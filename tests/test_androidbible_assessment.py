import json
import os
import tempfile
import threading
from types import SimpleNamespace
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import assess_androidbible as assessment


def result(code=0, stderr='', emitted=True, **kwargs):
    return {'exit_code': code, 'stderr': stderr, 'emitted_classes_sha256': {'Value.class': 'hash'} if emitted else {}, **kwargs}


class AndroidBibleAssessmentTests(unittest.TestCase):
    def diagnostic(self, category='unsupported_language', code='VALUE_CLASS_JVM_BOXING_UNSUPPORTED', message='Unrelated wording'):
        return {'source_identity': 'src/λ.kt', 'offset': 19, 'category': category, 'code': code, 'message': message, 'path': '/original/src/λ.kt', 'line': 3, 'column': 5}

    def test_structured_code_classification_does_not_depend_on_message(self):
        for code in assessment.UNSUPPORTED_DIAGNOSTIC_CODES:
            for message in ['Unsupported value class', 'Different wording with no classification keyword']:
                with self.subTest(code=code, message=message):
                    diagnostic = self.diagnostic(code=code, message=message)
                    self.assertEqual(assessment.classify(result(1, json.dumps(diagnostic)), result()), 'unsupported_language')
        ordinary = self.diagnostic(code='FUTURE_LANGUAGE_BOUNDARY', message='No English marker')
        self.assertEqual(assessment.classify(result(1, json.dumps(ordinary)), result()), 'unsupported_language')

    def test_structured_location_preserves_byte_offset_identity_and_readability(self):
        diagnostic = self.diagnostic(message='A readable explanation')
        parsed = assessment.structured_diagnostic(json.dumps(diagnostic, ensure_ascii=False) + '\n')
        self.assertEqual(parsed, diagnostic)
        first = assessment.first_diagnostic(json.dumps(diagnostic))
        self.assertEqual(first['message'], 'A readable explanation')
        self.assertEqual(first['offset'], 19)
        self.assertEqual(first['source_identity'], 'src/λ.kt')
        self.assertEqual((first['path'], first['line'], first['column']), ('/original/src/λ.kt', 3, 5))
        self.assertEqual(first['code'], 'VALUE_CLASS_JVM_BOXING_UNSUPPORTED')

    def test_structured_unclassified_fallback_uses_message_not_identity(self):
        diagnostic = self.diagnostic(category='unclassified', code='UNCLASSIFIED', message='Type mismatch')
        diagnostic['path'] = 'unsupported/only_requires.kt'
        self.assertEqual(assessment.classify(result(1, json.dumps(diagnostic)), result()), 'compiler_failure')
        diagnostic['message'] = 'Unsupported declaration'
        self.assertEqual(assessment.classify(result(1, json.dumps(diagnostic)), result()), 'unsupported_language')

    def test_structured_categories_keep_reference_invalid_dependency_disambiguation(self):
        unsupported = result(1, json.dumps(self.diagnostic()))
        self.assertEqual(assessment.classify(unsupported, result(1, 'A.kt:1:1: error: Unresolved reference: Helper')), 'dependency_failure')
        self.assertEqual(assessment.classify(unsupported, result(1, 'A.kt:1:1: error: Conflicting declarations')), 'invalid_kotlin')
        for category in ['missing_dependency', 'invalid_kotlin']:
            diagnostic = self.diagnostic(category=category, code='SPECIFIC_CODE')
            self.assertEqual(assessment.classify(result(1, json.dumps(diagnostic)), result()), 'compiler_failure')
        for category in ['setup', 'io']:
            diagnostic = self.diagnostic(category=category, code='SPECIFIC_CODE')
            self.assertEqual(assessment.classify(result(1, json.dumps(diagnostic)), result()), 'setup_failure')
            self.assertEqual(assessment.classify(result(1, json.dumps(diagnostic)), result(2)), 'compiler_failure')

    def test_structured_schema_rejects_incomplete_or_invalid_records(self):
        self.assertIsNone(assessment.structured_diagnostic('{}'))
        self.assertIsNone(assessment.structured_diagnostic('not JSON'))
        self.assertIsNone(assessment.structured_diagnostic(json.dumps([self.diagnostic()])))
        for key, invalid in [('offset', -1), ('offset', True), ('category', []), ('category', 'unknown'), ('code', ''), ('source_identity', 1), ('column', 0)]:
            diagnostic = self.diagnostic()
            diagnostic[key] = invalid
            with self.subTest(key=key, invalid=invalid):
                self.assertIsNone(assessment.structured_diagnostic(json.dumps(diagnostic)))
        malformed = self.diagnostic(message='Unsupported declaration')
        malformed['offset'] = -1
        self.assertEqual(assessment.classify(result(1, json.dumps(malformed)), result()), 'compiler_failure')
        minimal = {'source_identity': None, 'offset': 0, 'category': 'setup', 'code': 'MISSING_OUTPUT', 'message': 'Provide output'}
        first = assessment.first_diagnostic(json.dumps(minimal))
        self.assertIsNone(first['path'])
        self.assertIsNone(first['line'])
        self.assertEqual(first['offset'], 0)

    def test_emission_requires_classes_and_matching_oracle_acceptance(self):
        self.assertEqual(assessment.classify(result(), result()), 'emitted')
        self.assertEqual(assessment.classify(result(emitted=False), result()), 'compiler_failure')
        self.assertEqual(assessment.classify(result(), result(1, 'Value.kt:2:3: error: Type mismatch')), 'compiler_failure')

    def test_valid_unsupported_and_invalid_kotlin_are_distinct(self):
        candidate = result(1, 'Value.kt:2:3: error: Unsupported value class', emitted=False)
        self.assertEqual(assessment.classify(candidate, result()), 'unsupported_language')
        self.assertEqual(assessment.classify(candidate, result(1, 'Value.kt:2:3: error: Expecting member declaration')), 'invalid_kotlin')
        self.assertEqual(assessment.classify(result(1, 'Value.kt:2:3: error: Type mismatch'), result()), 'compiler_failure')

    def test_dependency_gap_never_claims_intrinsic_invalidity(self):
        self.assertEqual(assessment.classify(result(1, 'Expected identifier'), result(1, 'Value.kt:5:4: error: Unresolved reference: Helper')), 'dependency_failure')
        self.assertEqual(assessment.classify(result(1, 'Unresolved library class Helper'), result()), 'compiler_failure')
        self.assertEqual(assessment.classify(result(1, 'Unsupported imported declaration Helper'), result()), 'unsupported_language')
        self.assertEqual(assessment.classify(result(), result(), setup_error='Missing Android SDK'), 'setup_failure')

    def test_timeout_launch_and_compiler_crash_are_distinct(self):
        self.assertEqual(assessment.classify(result(None, timed_out=True), result()), 'timeout')
        self.assertEqual(assessment.classify(result(None, launch_error='missing binary'), result()), 'setup_failure')
        self.assertEqual(assessment.classify(result(-9), result()), 'compiler_failure')

    def test_first_diagnostic_keeps_original_location_and_multiline_body(self):
        diagnostic = assessment.first_diagnostic('warning: x\n/workspace/A.kt:27:16: error: Unsupported declaration\n    val x = 1\n')
        self.assertEqual(diagnostic, {'path': '/workspace/A.kt', 'line': 27, 'column': 16, 'message': 'Unsupported declaration'})
        self.assertEqual(assessment.first_diagnostic('e: file:///workspace/A.kt: (5, 9): Missing type')['line'], 5)
        self.assertIsNone(assessment.first_diagnostic('Compiler crashed')['line'])

    def test_source_groups_include_tests_and_tools(self):
        self.assertEqual(assessment.source_group('Module/src/main/java/Value.kt'), 'production')
        self.assertEqual(assessment.source_group('Module/src/test/java/Value.kt'), 'tests')
        self.assertEqual(assessment.source_group('Module/src/androidTest/kotlin/Value.kt'), 'tests')
        self.assertEqual(assessment.source_group('tools/script/Value.kt'), 'tools')

    def test_counts_derive_from_cases_and_preserve_not_run_behavior(self):
        cases = [{'outcome': 'emitted', 'group': 'production', 'checks': {'emission': {'outcome': 'emitted'}, 'behavior': {'outcome': 'not_run'}}}, {'outcome': 'dependency_failure', 'group': 'tests', 'candidate': {'diagnostic': self.diagnostic(category='missing_dependency', code='MISSING_TYPE')}, 'checks': {'emission': {'outcome': 'not_emitted'}, 'behavior': {'outcome': 'not_run'}}}]
        counts = assessment.derived_counts(cases)
        self.assertEqual(sum(counts['outcomes'].values()), len(cases))
        self.assertEqual(counts['groups']['tests'], {'dependency_failure': 1})
        self.assertEqual(counts['behavior'], {'not_run': 2})
        self.assertEqual(counts['diagnostic_codes'], {'MISSING_TYPE': 1})
        self.assertEqual(counts['diagnostic_categories'], {'missing_dependency': 1})

    def test_resolved_model_selects_exact_source_and_release_task(self):
        source = assessment.SNAPSHOT / 'Alkitab/src/main/java/Value.kt'
        task = {'sources': ['/disposable/androidbible/Alkitab/src/main/java/Value.kt'], 'compile_classpath': ['/dependency.jar']}
        model = {'modules': {'Alkitab': {'compilation_tasks': {'compileDebugKotlin': task, 'compileReleaseKotlin': task}}}}
        _, name, selected = assessment.resolved_task(model, source)
        self.assertEqual(name, 'compileReleaseKotlin')
        self.assertEqual(selected['compile_classpath'], ['/dependency.jar'])
        with self.assertRaisesRegex(ValueError, 'No resolved'):
            assessment.resolved_task(model, source.with_name('Missing.kt'))

    def test_partial_capture_flags_cannot_be_treated_as_complete_inputs(self):
        module = {'setup_failures': [{'field': 'compilePlainReleaseKotlin.sources', 'diagnostic': 'BuildConfig producer has not completed'}, {'field': 'compilePlainDebugUnitTestKotlin.sources', 'diagnostic': 'KSP producer has not completed'}]}
        task = {'source_collection_incomplete': True, 'classpath_incomplete': False}
        failures = assessment.resolved_setup_errors(module, 'compilePlainReleaseKotlin', task)
        self.assertEqual(len(failures), 2)
        self.assertIn('source_collection_incomplete', failures[0]['field'])
        self.assertIn('BuildConfig', failures[1]['diagnostic'])
        self.assertEqual(assessment.resolved_setup_errors(module, 'compileReleaseKotlin', {}), [])
        self.assertTrue(assessment.resolved_setup_errors({}, 'compileReleaseKotlin', {'classpath_incomplete': True}))

    def test_test_tasks_require_explicit_authoritative_friend_capture(self):
        name = 'compileDebugUnitTestKotlin'
        failures = assessment.resolved_setup_errors({}, name, {'role': 'test'})
        self.assertTrue(any(entry['field'].endswith('.friend_paths') for entry in failures))
        complete = {'role': 'test', 'friend_paths': [], 'friend_paths_provenance': {'captured': True, 'api': 'BaseKotlinCompile.friendPaths'}}
        self.assertEqual(assessment.resolved_setup_errors({}, name, complete), [])
        incomplete = {**complete, 'friend_paths_incomplete': True}
        self.assertTrue(assessment.resolved_setup_errors({}, name, incomplete))
        incomplete = {**complete, 'friend_paths_provenance': {'captured': False}}
        self.assertTrue(assessment.resolved_setup_errors({}, name, incomplete))
        self.assertEqual(assessment.resolved_setup_errors({}, 'compileReleaseKotlin', {}), [])

    def resolved_assessment(self, directory, friend_paths):
        source = assessment.SNAPSHOT / 'AlkitabYes2/src/test/java/yuku/alkitab/yes2/Yes2RoundTripTest.kt'
        self.assertTrue(source.is_file())
        dependency = directory / 'dependencies'
        dependency.mkdir(exist_ok=True)
        task = {'role': 'test', 'sources': [str(source)], 'compile_classpath': [str(dependency)], 'compiler_options': {'module_name': 'AlkitabYes2_test'}, 'friend_paths': list(map(str, friend_paths)), 'friend_paths_incomplete': False, 'friend_paths_provenance': {'captured': True, 'api': 'BaseKotlinCompile.friendPaths'}}
        harness = assessment.Assessment.__new__(assessment.Assessment)
        harness.args = SimpleNamespace(mode='resolved', compiler=Path('/mock/korrust'), official_timeout=30)
        harness.model = {'modules': {'AlkitabYes2': {'compilation_tasks': {'compileDebugUnitTestKotlin': task}}}}
        harness.lock = threading.Lock()
        harness.validated_classpaths = {}
        harness.classpath_inputs = {}
        harness.work = directory / 'work'
        harness.kotlinc = Path('/mock/kotlinc')
        harness.regression = None
        return harness, source, task

    def test_resolved_friend_paths_are_replayed_in_each_compiler_cli_format_and_hashed(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            friends = [directory / 'production-kotlin', directory / 'production-java']
            for path in friends:
                path.mkdir()
                (path / 'owned.class').write_bytes(b'owned production bytes')
            harness, source, _ = self.resolved_assessment(directory, friends)
            commands = []
            def run(label, argv, timeout=None):
                commands.append(list(map(str, argv)))
                output = Path(argv[-1])
                output.mkdir(parents=True)
                (output / 'test.class').write_bytes(b'mocked emitted bytes')
                return {'exit_code': 0, 'stdout': '', 'stderr': '', 'command_index': len(commands) - 1}
            harness.run = run
            case = harness.probe((0, source))
            self.assertEqual(case['outcome'], 'emitted')
            candidate, official = commands
            self.assertEqual(candidate[candidate.index('--friend-paths') + 1], os.pathsep.join(map(str, friends)))
            self.assertIn('-Xfriend-paths=' + ','.join(map(str, friends)), official)
            self.assertEqual(case['input_context']['friend_paths'], list(map(str, friends)))
            self.assertEqual(case['input_context']['friend_paths_provenance']['api'], 'BaseKotlinCompile.friendPaths')
            for path in friends:
                self.assertEqual(case['input_context']['friend_paths_inputs'][str(path.resolve())]['files_sha256'], {'owned.class': assessment.sha(path / 'owned.class')})

    def test_missing_friend_outputs_and_old_test_models_stop_before_compilers(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            missing = directory / 'missing-production-output'
            harness, source, task = self.resolved_assessment(directory, [missing])
            def forbidden(*args, **kwargs):
                self.fail('Compiler invoked with incomplete friend context')
            harness.run = forbidden
            case = harness.probe((0, source))
            self.assertEqual(case['outcome'], 'setup_failure')
            self.assertEqual(case['checks']['emission']['outcome'], 'not_run')
            self.assertIn(str(missing), case['first_diagnostic']['message'])
            task.pop('friend_paths')
            task.pop('friend_paths_provenance')
            task['compiler_options']['module_name'] = 'AlkitabYes2'
            case = harness.probe((0, source))
            self.assertEqual(case['outcome'], 'setup_failure')
            self.assertIn('captured friend paths', case['first_diagnostic']['message'])

    def test_explicit_empty_friend_capture_does_not_hide_invalid_internal_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            harness, source, _ = self.resolved_assessment(Path(temporary), [])
            commands = []
            def run(label, argv, timeout=None):
                commands.append(list(map(str, argv)))
                return {'exit_code': 1, 'stdout': '', 'stderr': 'A.kt:1:1: error: Cannot access internal declaration', 'command_index': len(commands) - 1}
            harness.run = run
            case = harness.probe((0, source))
            self.assertEqual(case['outcome'], 'invalid_kotlin')
            self.assertEqual(len(commands), 2)
            self.assertNotIn('--friend-paths', commands[0])
            self.assertFalse(any(argument.startswith('-Xfriend-paths=') for argument in commands[1]))

    def test_flavored_production_source_prefers_release_variant(self):
        source = assessment.SNAPSHOT / 'Alkitab/src/main/java/Value.kt'
        task = {'sources': ['/disposable/androidbible/Alkitab/src/main/java/Value.kt'], 'compile_classpath': []}
        model = {'modules': {'Alkitab': {'compilation_tasks': {'compilePlainDebugKotlin': task, 'compilePlainReleaseKotlin': task}}}}
        self.assertEqual(assessment.resolved_task(model, source)[1], 'compilePlainReleaseKotlin')

    def test_gradle_options_keep_language_target_plugin_boundary_inputs(self):
        options = assessment.official_options({'jvmTarget': '21', 'languageVersion': '2.3', 'apiVersion': '2.3', 'javaParameters': 'true', 'allWarningsAsErrors': 'false', 'module_name': 'selected', 'freeCompilerArgs': ['-Xjvm-default=all']})
        self.assertEqual(options, ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '21', '-module-name', 'selected', '-java-parameters', '-Xjvm-default=all'])

    def test_gradle_enum_values_from_captured_model_become_cli_values(self):
        options = assessment.official_options({'jvmTarget': 'JVM_21', 'languageVersion': 'KOTLIN_2_3', 'apiVersion': 'KOTLIN_2_3', 'jvmDefault': 'NO_COMPATIBILITY'})
        self.assertEqual(options, ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '21', '-jvm-default', 'no-compatibility'])
        self.assertEqual(assessment.official_options({'jvmTarget': 'JVM_1_8', 'languageVersion': None, 'apiVersion': None}), ['-jvm-target', '1.8'])

    def test_comparison_preserves_changed_stable_codes_without_message_change(self):
        before = assessment.first_diagnostic(json.dumps(self.diagnostic()))
        after = assessment.first_diagnostic(json.dumps(self.diagnostic(code='ENUM_ENTRY_BODY_UNSUPPORTED')))
        previous = {'cases': [{'id': 'A.kt', 'source_sha256': 'same', 'outcome': 'unsupported_language', 'first_diagnostic': before}]}
        current = [{'id': 'A.kt', 'source_sha256': 'same', 'outcome': 'unsupported_language', 'first_diagnostic': after}]
        changes = assessment.compare(previous, current, {}, 'same')['changes']
        self.assertEqual(changes[0]['before']['first_diagnostic']['code'], 'VALUE_CLASS_JVM_BOXING_UNSUPPORTED')
        self.assertEqual(changes[0]['after']['first_diagnostic']['code'], 'ENUM_ENTRY_BODY_UNSUPPORTED')

    def test_comparison_names_compiler_and_classpath_changes(self):
        previous = {'compiler': {'binary_sha256': 'old'}, 'classpath_scope': 'limited', 'cases': [{'id': 'A.kt', 'source_sha256': 'same', 'outcome': 'rejected', 'diagnostic': 'A.kt:3:4: error: Missing class'}]}
        cases = [{'id': 'A.kt', 'source_sha256': 'same', 'outcome': 'unsupported_language', 'first_diagnostic': {'path': 'A.kt', 'line': 4, 'column': 5, 'message': 'Unsupported Double'}}]
        comparison = assessment.compare(previous, cases, {'binary_sha256': 'new'}, 'resolved Gradle')
        self.assertEqual(comparison['prior_classpath_scope'], 'limited')
        self.assertEqual(comparison['current_classpath_scope'], 'resolved Gradle')
        self.assertEqual(comparison['current_compiler']['binary_sha256'], 'new')
        self.assertTrue(comparison['changes'][0]['same_source'])
        self.assertEqual(comparison['changes'][0]['after']['first_diagnostic']['line'], 4)


if __name__ == '__main__':
    unittest.main()
