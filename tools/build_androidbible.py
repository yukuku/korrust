import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from classpath_policy import validate_classpath


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "benchmarks/sources/androidbible"
MODULE = "AlkitabYes2"


INIT_SCRIPT = r'''
import groovy.json.JsonOutput
import groovy.json.JsonSlurper

gradle.projectsEvaluated {
    def moduleNames = new JsonSlurper().parseText(System.getenv('KORRUST_BUILD_MODULES'))
    def moduleArtifacts = new JsonSlurper().parseText(System.getenv('KORRUST_BUILD_MODULE_ARTIFACTS'))
    moduleNames.each { moduleName ->
        def selected = gradle.rootProject.project(":" + moduleName)
        def artifacts = new File(moduleArtifacts[moduleName])
        def selectedTasks = ['compileDebugKotlin', 'compileReleaseKotlin']
        def orderedPaths = { collection -> collection.files.collect { it.absolutePath } }
        def optionValue = { value ->
            def resolved = value instanceof org.gradle.api.provider.Provider ? value.orNull : value
            if (resolved == null) return null
            if (resolved instanceof Collection) return resolved.collect { it.toString() }
            return resolved.toString()
        }
        def compilerOptions = { task ->
            def result = [:]
            ['jvmTarget', 'languageVersion', 'apiVersion', 'freeCompilerArgs', 'javaParameters',
             'allWarningsAsErrors', 'suppressWarnings', 'progressiveMode', 'optIn', 'jvmDefault'].each { name ->
                if (task.compilerOptions.hasProperty(name)) result[name] = optionValue(task.compilerOptions."${name}")
            }
            result.module_name = task.moduleName.get()
            result
        }
        def friendInputs = { task ->
            def provenance = [
                api: 'org.jetbrains.kotlin.gradle.tasks.BaseKotlinCompile.getFriendPaths',
                compiler_consumer: 'org.jetbrains.kotlin.gradle.tasks.KotlinCompile: args.friendPaths = friendPaths.toPathsArray()',
                path_order: 'Gradle FileCollection iteration; normalize().absolutePath; no existence filtering',
                task_class: task.class.name,
                captured: false,
            ]
            try {
                def apiClass = task.class.classLoader.loadClass('org.jetbrains.kotlin.gradle.tasks.BaseKotlinCompile')
                provenance.api_code_source = new File(apiClass.protectionDomain.codeSource.location.toURI()).absolutePath
            } catch (Exception ignored) {
                provenance.api_code_source = null
            }
            try {
                def paths = task.friendPaths.files.collect { it.toPath().normalize().toAbsolutePath().toString() }
                provenance.captured = true
                return [friend_paths: paths, friend_paths_incomplete: false, friend_paths_provenance: provenance]
            } catch (Exception error) {
                return [friend_paths: [], friend_paths_incomplete: true,
                    friend_paths_error: error.message, friend_paths_provenance: provenance]
            }
        }
        def sourceSets = {
            selected.extensions.getByName('android').sourceSets.collectEntries { sourceSet ->
                def kotlin = sourceSet.hasProperty('kotlin') ? sourceSet.kotlin.srcDirs.collect { it.absolutePath } : []
                [(sourceSet.name): [name: sourceSet.name,
                    role: sourceSet.name.toLowerCase().contains('test') ? 'test' : 'production',
                    java_roots: sourceSet.java.srcDirs.collect { it.absolutePath }.sort(),
                    kotlin_roots: kotlin.sort(),
                    resource_roots: sourceSet.resources.srcDirs.collect { it.absolutePath }.sort(),
                    manifest: sourceSet.manifest.srcFile.absolutePath]]
            }
        }
        def compilationModel = { taskName ->
            def task = selected.tasks.named(taskName).get()
            def files = task.sources.files.findAll { it.name.endsWith('.kt') || it.name.endsWith('.java') }
            def sourceRoot = new File(selected.projectDir, 'src').absolutePath + File.separator
            def generated = files.findAll { !it.absolutePath.startsWith(sourceRoot) }
            def result = [task: task.path,
                role: taskName.toLowerCase().contains('test') ? 'test' : 'production',
                sources: files.collect { it.absolutePath }.sort(),
                source_roots: files.collect { it.parentFile.absolutePath }.unique().sort(),
                generated_source_roots: generated.collect { it.parentFile.absolutePath }.unique().sort(),
                compile_classpath: orderedPaths(task.libraries),
                compiler_options: compilerOptions(task),
                compiler_plugin_classpath: orderedPaths(task.pluginClasspath),
                compiler_plugin_options: task.pluginOptions.get().collectMany { config -> config.allOptions().collectMany { id, options -> options.collect { option -> 'plugin:' + id + ':' + option.key + '=' + option.value } } }]
            result.putAll(friendInputs(task))
            result
        }

        if (System.getenv('KORRUST_CAPTURE_ONLY') == '1') {
            selected.tasks.register('korrustCaptureModel') {
                doLast {
                    def failures = []
                    def safely = { field, action, fallback ->
                        try { action() }
                        catch (Exception error) {
                            failures << [field: field, diagnostic: error.message]
                            fallback
                        }
                    }
                    def prefix = selected.name == 'Alkitab' ? 'Plain' : ''
                    def names = ["compile${prefix}DebugKotlin", "compile${prefix}ReleaseKotlin", "compile${prefix}DebugUnitTestKotlin", "compile${prefix}ReleaseUnitTestKotlin"]
                    def captured = [:]
                    names.each { name ->
                        def task = selected.tasks.findByName(name)
                        if (task == null) {
                            failures << [field: name, diagnostic: 'Required selected variant task is missing']
                        } else {
                            def configurationName = name.substring('compile'.length(), name.length() - 'Kotlin'.length())
                            configurationName = configurationName[0].toLowerCase() + configurationName.substring(1) + 'CompileClasspath'
                            def sourceFiles = safely(name + '.sources', { task.sources.files.findAll { it.name.endsWith('.kt') || it.name.endsWith('.java') }.collect { it.absolutePath }.sort() }, null)
                            def isSourceIncomplete = sourceFiles == null
                            def isTest = name.toLowerCase().contains('test')
                            def buildType = name.contains('Release') ? 'Release' : 'Debug'
                            def sourceSetNames = isTest ? ['test', 'test' + buildType] : ['main', buildType.toLowerCase()]
                            if (prefix) sourceSetNames += isTest ? ['test' + prefix, 'test' + prefix + buildType] : [prefix.toLowerCase(), prefix.toLowerCase() + buildType]
                            def selectedSets = sourceSets().findAll { key, value -> sourceSetNames.contains(key) }
                            def declaredRoots = selectedSets.values().collectMany { value -> value.java_roots + value.kotlin_roots }.unique().sort()
                            if (sourceFiles == null) sourceFiles = declaredRoots.collectMany { directory -> selected.fileTree(directory) { include '**/*.kt', '**/*.java' }.files.collect { it.absolutePath } }.unique().sort()
                            def paths = safely(name + '.compile_classpath', { orderedPaths(task.libraries) }, null)
                            def isIncomplete = paths == null
                            if (paths == null) {
                                def configuration = selected.configurations.findByName(configurationName)
                                if (configuration != null) {
                                    paths = safely(name + '.partial_component_classpath', {
                                        def view = configuration.incoming.artifactView {
                                            lenient = true
                                            attributes { attribute(org.gradle.api.attributes.Attribute.of('artifactType', String), 'android-classes-jar') }
                                        }.artifacts
                                        view.failures.each { error -> failures << [field: name + '.partial_component_classpath', diagnostic: error.message] }
                                        orderedPaths(view.artifactFiles)
                                    }, [])
                                } else { paths = [] }
                            }
                            def generatedOutputs = safely(name + '.source_generation_dependencies', {
                                task.sources.buildDependencies.getDependencies(task).collect { producer -> [task: producer.path, outputs: orderedPaths(producer.outputs.files)] }
                            }, [])
                            captured[name] = [
                                task: task.path,
                                role: name.toLowerCase().contains('test') ? 'test' : 'production',
                                sources: sourceFiles,
                                source_roots: declaredRoots,
                                source_collection_incomplete: isSourceIncomplete,
                                source_collection_scope: isSourceIncomplete ? 'declared selected Android source sets; generated source providers unavailable' : 'actual Kotlin task sources',
                                selected_source_sets: sourceSetNames,
                                generated_source_roots: sourceFiles.findAll { !it.startsWith(new File(selected.projectDir, 'src').absolutePath + File.separator) }.collect { new File(it).parentFile.absolutePath }.unique().sort(),
                                source_generation_dependencies: generatedOutputs,
                                compile_classpath: paths,
                                classpath_incomplete: isIncomplete,
                                classpath_scope: isIncomplete ? 'partial resolved component artifacts; task classpath resolution failed' : 'actual selected Kotlin task libraries',
                                compiler_options: safely(name + '.compiler_options', { compilerOptions(task) }, [:]),
                                compiler_plugin_classpath: safely(name + '.compiler_plugin_classpath', { orderedPaths(task.pluginClasspath) }, []),
                                compiler_plugin_options: safely(name + '.compiler_plugin_options', {
                                    task.pluginOptions.get().collectMany { config -> config.allOptions().collectMany { id, options -> options.collect { option -> 'plugin:' + id + ':' + option.key + '=' + option.value } } }
                                }, []),
                            ]
                            captured[name].putAll(friendInputs(task))
                        }
                    }
                    def release = selected.tasks.findByName("compile${prefix}ReleaseKotlin")
                    def runtime = [:]
                    ["${prefix.toLowerCase()}DebugRuntimeClasspath", "${prefix.toLowerCase()}ReleaseRuntimeClasspath"].each { rawName ->
                        def name = prefix ? rawName : rawName[0].toLowerCase() + rawName.substring(1)
                        def configuration = selected.configurations.findByName(name)
                        if (configuration != null) {
                            runtime[name] = safely(name, {
                                def view = configuration.incoming.artifactView {
                                    lenient = true
                                    attributes { attribute(org.gradle.api.attributes.Attribute.of('artifactType', String), 'android-classes-jar') }
                                }.artifacts
                                view.failures.each { error -> failures << [field: name, diagnostic: error.message] }
                                orderedPaths(view.artifactFiles)
                            }, [])
                        }
                    }
                    def compilerVersion = release == null ? null : safely('resolved_kotlin_compiler_version', { release.'getKotlinCompilerVersion$kotlin_gradle_plugin_common'().get().toString() }, null)
                    def compilerPaths = release == null ? [] : safely('resolved_kotlin_compiler_classpath', { orderedPaths(release.'getDefaultCompilerClasspath$kotlin_gradle_plugin_common'()) }, [])
                    def model = [schema_version: 2, module: selected.name, capture_only: true,
                        source_sets: safely('source_sets', { sourceSets() }, [:]),
                        compilation_tasks: captured, runtime_classpaths: runtime,
                        classpath: captured["compile${prefix}ReleaseKotlin"]?.compile_classpath ?: [],
                        resolved_kotlin_compiler_version: compilerVersion,
                        resolved_kotlin_compiler_classpath: compilerPaths,
                        setup_failures: failures,
                        dependency_boundary: [selected_modules: moduleNames,
                            candidate_owned_sources: captured.findAll { name, value -> value.role == 'production' }.collectMany { name, value -> value.sources.findAll { it.endsWith('.kt') } }.unique().sort(),
                            production_compilation_executed: false,
                            ownership: 'Input capture only. No selected production source was compiled by either compiler.',
                            dependencies_compiler: 'Existing sibling outputs, if present, retain their recorded toolchain; no sibling compile task executed by capture.'],
                        plugins: [gradle: gradle.gradleVersion, applied_plugin_classes: selected.plugins.collect { it.class.name }.sort()],
                        options: [selected_variants: names, input_capture_only: true],
                    ]
                    new File(artifacts, 'gradle-model.json').text = JsonOutput.prettyPrint(JsonOutput.toJson(model))
                }
            }
        } else {
        if (System.getenv('KORRUST_BUILD_MODE') == 'korrust') {
            selectedTasks.each { taskName ->
                selected.tasks.named(taskName).configure { task ->
                    task.setActions([])
                    task.outputs.upToDateWhen { false }
                    task.outputs.cacheIf { false }
                    task.doLast {
                        def request = [
                            module: selected.name,
                            task: task.path,
                            compiler: System.getenv('KORRUST_BUILD_COMPILER'),
                            sources: task.sources.files.findAll { it.name.endsWith('.kt') }.collect { it.absolutePath }.sort(),
                            classpath: orderedPaths(task.libraries),
                            module_name: task.moduleName.get(),
                            source_roots: compilationModel(taskName).source_roots,
                            generated_source_roots: compilationModel(taskName).generated_source_roots,
                            compiler_options: compilerOptions(task),
                            compiler_plugin_classpath: compilationModel(taskName).compiler_plugin_classpath,
                            compiler_plugin_options: compilationModel(taskName).compiler_plugin_options,
                            destination: task.destinationDirectory.get().asFile.absolutePath,
                            stage: new File(selected.layout.buildDirectory.get().asFile, "tmp/korrust/${taskName}").absolutePath,
                            work_dir: selected.rootDir.absolutePath,
                            trace: new File(artifacts, "${taskName}.json").absolutePath,
                        ]
                        def requestFile = new File(artifacts, "${taskName}-request.json")
                        requestFile.text = JsonOutput.prettyPrint(JsonOutput.toJson(request))
                        def process = new ProcessBuilder([
                            System.getenv('KORRUST_BUILD_PYTHON'),
                            System.getenv('KORRUST_BUILD_TOOL'),
                            '--compile-request', requestFile.absolutePath,
                        ]).directory(selected.rootDir).redirectErrorStream(true).start()
                        process.inputStream.withReader { reader ->
                            reader.eachLine { line -> selected.logger.lifecycle(line) }
                        }
                        if (process.waitFor() != 0) {
                            throw new GradleException("Korrust failed for ${task.path}; official production Kotlin fallback is disabled")
                        }
                    }
                }
            }
        }
        selected.tasks.register('korrustBuildModel') {
            mustRunAfter selected.tasks.named('testDebugUnitTest')
            mustRunAfter selected.tasks.named('bundleReleaseAar')
            doLast {
                def releaseKotlin = selected.tasks.named('compileReleaseKotlin').get()
                def debugKotlin = selected.tasks.named('compileDebugKotlin').get()
                def releaseJava = selected.tasks.named('compileReleaseJavaWithJavac').get()
                def testTask = selected.tasks.named('testDebugUnitTest').get()
                def unitKotlin = selected.tasks.named('compileDebugUnitTestKotlin').get()
                def model = [
                    schema_version: 2,
                    module: selected.name,
                    source_sets: sourceSets(),
                    compilation_tasks: ['compileDebugKotlin', 'compileReleaseKotlin', 'compileDebugUnitTestKotlin'].collectEntries { name -> [(name): compilationModel(name)] },
                    generated_source_roots: ['compileDebugKotlin', 'compileReleaseKotlin', 'compileDebugUnitTestKotlin'].collectMany { name -> compilationModel(name).generated_source_roots }.unique().sort(),
                    dependency_boundary: [
                        selected_modules: moduleNames,
                        candidate_owned_tasks: selectedTasks.collect { selected.path + ':' + it },
                        candidate_owned_sources: selectedTasks.collectMany { name -> compilationModel(name).sources.findAll { it.endsWith('.kt') } }.unique().sort(),
                        official_test_tasks: ['compileDebugUnitTestKotlin'],
                        external_project_components: selected.configurations.findAll { it.canBeResolved && it.name in ['releaseCompileClasspath', 'debugCompileClasspath', 'debugUnitTestCompileClasspath', 'debugUnitTestRuntimeClasspath'] }.collectMany { configuration ->
                            configuration.incoming.resolutionResult.allComponents.findAll { it.id instanceof org.gradle.api.artifacts.component.ProjectComponentIdentifier && !moduleNames.contains(it.id.projectPath.tokenize(':').last()) }.collect { it.id.displayName }
                        }.unique().sort(),
                        external_compiler: 'normal Gradle Kotlin/Java/NDK compilers for unselected modules',
                    ],
                    plugins: [
                        android: selected.plugins.findPlugin('com.android.library').class.classLoader.loadClass('com.android.Version').getField('ANDROID_GRADLE_PLUGIN_VERSION').get(null),
                        gradle: gradle.gradleVersion,
                        applied_plugin_classes: selected.plugins.collect { it.class.name }.sort(),
                    ],
                    runtime_classpaths: [
                        releaseRuntimeClasspath: orderedPaths(selected.configurations.getByName('releaseRuntimeClasspath').incoming.artifactView { attributes { attribute(org.gradle.api.attributes.Attribute.of('artifactType', String), 'android-classes-jar') } }.files),
                        debugRuntimeClasspath: orderedPaths(selected.configurations.getByName('debugRuntimeClasspath').incoming.artifactView { attributes { attribute(org.gradle.api.attributes.Attribute.of('artifactType', String), 'android-classes-jar') } }.files),
                        debugUnitTestRuntimeClasspath: orderedPaths(testTask.classpath),
                    ],
                    kotlin_classes: releaseKotlin.destinationDirectory.get().asFile.absolutePath,
                    java_classes: releaseJava.destinationDirectory.get().asFile.absolutePath,
                    debug_kotlin_classes: debugKotlin.destinationDirectory.get().asFile.absolutePath,
                    classpath: orderedPaths(releaseKotlin.libraries),
                    java_classpath: orderedPaths(releaseJava.classpath),
                    test_runtime_classpath: orderedPaths(testTask.classpath),
                    test_classes: testTask.testClassesDirs.files.collect { it.absolutePath }.sort(),
                    test_results: testTask.reports.junitXml.outputLocation.get().asFile.absolutePath,
                    aar: new File(selected.layout.buildDirectory.get().asFile, "outputs/aar/${selected.name}-release.aar").absolutePath,
                    resolved_kotlin_compiler_version: releaseKotlin.'getKotlinCompilerVersion$kotlin_gradle_plugin_common'().get().toString(),
                    resolved_kotlin_compiler_classpath: releaseKotlin.'getDefaultCompilerClasspath$kotlin_gradle_plugin_common'().files.collect { it.absolutePath }.sort(),
                    options: [
                        kotlin_jvm_target: releaseKotlin.compilerOptions.jvmTarget.get().target,
                        kotlin_module_name: releaseKotlin.moduleName.get(),
                        java_source_compatibility: releaseJava.sourceCompatibility,
                        java_target_compatibility: releaseJava.targetCompatibility,
                        unit_test_friend_paths: friendInputs(unitKotlin).friend_paths,
                        production_tasks: selectedTasks,
                        unit_test_compiler: 'official Kotlin Gradle plugin',
                        dependencies_compiler: 'normal Gradle Kotlin/Java/NDK compilers for unselected modules',
                    ],
                ]
                new File(artifacts, 'gradle-model.json').text = JsonOutput.prettyPrint(JsonOutput.toJson(model))
            }
        }
        }
    }
}
'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def resolved_inputs(model):
    paths = {}
    friend_context_failures = []
    for name, task in model.get("compilation_tasks", {}).items():
        is_test = task.get("role") == "test" or "test" in name.lower()
        provenance = task.get("friend_paths_provenance", {})
        if task.get("friend_paths_incomplete") or provenance.get("captured") is False:
            friend_context_failures.append({"field": name + ".friend_paths", "diagnostic": task.get("friend_paths_error") or "Gradle friend path capture is unavailable"})
        elif is_test and ("friend_paths" not in task or provenance.get("captured") is not True):
            friend_context_failures.append({"field": name + ".friend_paths", "diagnostic": "Test task has no authoritative captured Gradle friend context"})
        api_source = provenance.get("api_code_source")
        if api_source:
            paths.setdefault(api_source, []).append(name + ":friend_paths_api")
        for key in ("sources", "compile_classpath", "compiler_plugin_classpath", "friend_paths"):
            for item in task.get(key, []):
                paths.setdefault(item, []).append(name + ":" + key)
        for producer in task.get("source_generation_dependencies", []):
            for item in producer.get("outputs", []):
                paths.setdefault(item, []).append(name + ":source_generation:" + producer["task"])
    for item in model.get("resolved_kotlin_compiler_classpath", []):
        paths.setdefault(item, []).append("compiler")
    records = []
    for item, roles in sorted(paths.items()):
        path = Path(item)
        entry = {"path": item, "roles": roles, "exists": path.exists()}
        if path.is_file():
            entry.update(kind="file", sha256=digest(path))
        elif path.is_dir():
            entry.update(kind="directory", files_sha256={
                str(child.relative_to(path)): digest(child)
                for child in sorted(path.rglob("*")) if child.is_file()
            })
        else:
            entry["kind"] = "missing"
        records.append(entry)
    return {"entries": records,
            "friend_context_failures": friend_context_failures,
            "missing_required_inputs": [entry["path"] for entry in records if not entry["exists"]]}


def failure_category(log, has_model):
    lower = log.lower()
    setup_markers = ("sdk location not found", "failed to find platform sdk", "failed to find build tools",
                     "ndk not configured", "ndk side by side", "could not resolve", "could not get resource",
                     "could not download", "sdk package", "license for package", "pkix path building failed",
                     "unknownhostexception", "failed to install the following android sdk packages",
                     "plugin [", "no such file or directory")
    if any(marker in lower for marker in setup_markers):
        return "setup_failure"
    return "build_failure" if has_model else "model_or_build_failure"


def unpack_classpath(paths, destination=None):
    provenance = validate_classpath(paths)
    return [Path(item) for item in paths], provenance


def publication_files(directory):
    if not directory.exists():
        return {}
    files = {}
    for path in sorted(directory.rglob("*")):
        relative = path.relative_to(directory)
        if ".korrust" in relative.parts:
            continue
        if path.is_symlink():
            raise ValueError(f"Refusing symbolic link in Gradle publication: {path}")
        if path.is_file():
            files[str(relative)] = digest(path)
    return files


def preflight_publication(destination, manifest):
    previous = json.loads(manifest.read_text()) if manifest.exists() else {}
    for name in previous:
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts or "\\" in name or str(relative) != name:
            raise ValueError(f"Unsafe publication manifest output: {name}")
    existing = publication_files(destination)
    for name, checksum in existing.items():
        if name not in previous:
            raise ValueError(f"Refusing to overwrite unowned Gradle output: {destination / name}")
        if checksum != previous[name]:
            raise ValueError(f"Refusing to overwrite externally modified Gradle output: {destination / name}")
    for name in previous.keys() - existing.keys():
        raise ValueError(f"Owned Gradle output is missing: {destination / name}")
    return previous


def publish_stage(stage, destination, manifest):
    previous = preflight_publication(destination, manifest)
    current = publication_files(stage)
    for name in current:
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(f"Unsafe candidate output: {name}")
        target = destination / relative
        for parent in target.parents:
            if parent == destination:
                break
            if parent.exists() and not parent.is_dir():
                raise ValueError(f"Candidate output parent is not a directory: {parent}")
        if target.exists() and not target.is_file():
            raise ValueError(f"Candidate output is not a regular file: {target}")
    destination.mkdir(parents=True, exist_ok=True)
    for name in current:
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(stage / name, target)
    for name in previous.keys() - current.keys():
        (destination / name).unlink(missing_ok=True)
    write_json(manifest, current)


def validate_compile_options(request):
    options = request.get("compiler_options", {})
    free = options.get("freeCompilerArgs", [])
    if free:
        raise ValueError(f"Unassessed selected Kotlin compiler arguments: {free}")
    if request.get("compiler_plugin_classpath") or request.get("compiler_plugin_options"):
        raise ValueError("Selected Kotlin compiler plugins are unsupported")
    for name in ("languageVersion", "apiVersion", "jvmDefault"):
        if options.get(name) is not None:
            raise ValueError(f"Unassessed selected Kotlin compiler option: {name}={options[name]}")
    if str(options.get("javaParameters", "false")).lower() != "false":
        raise ValueError("Selected javaParameters option is unsupported")


def compile_request(path):
    request = json.loads(path.read_text())
    work = Path(request["work_dir"])
    module = request.get("module", MODULE)
    selected = work / module
    sources = [Path(item) for item in request["sources"]]
    if len(set(sources)) != len(sources):
        raise ValueError("Gradle production Kotlin source list contains duplicates")
    for source in sources:
        if not source.is_file() or source.suffix != ".kt" or not source.resolve().is_relative_to(selected.resolve()):
            raise ValueError(f"Invalid or missing selected module Kotlin source: {source}")
    roots = request.get("source_roots")
    if roots is None:
        roots = [str(selected / "src/main")]
    expected = sorted({str(source) for root in roots for source in Path(root).rglob("*.kt")})
    if sorted(request["sources"]) != expected:
        raise ValueError(f"Gradle production Kotlin sources differ from selected module inventory: {request['sources']} != {expected}")
    validate_compile_options(request)
    stage = Path(request["stage"])
    destination = Path(request["destination"])
    manifest = stage.parent / (stage.name + "-publication.json")
    preflight_publication(destination, manifest)
    directories, provenance = unpack_classpath(request["classpath"])
    for item in directories:
        if item.resolve() == destination.resolve() or item.resolve() == stage.resolve():
            raise ValueError("Candidate-owned output must not appear on production compilation classpath")
    trace = {"schema_version": 2, "module": module, "module_name": request.get("module_name", module),
             "task": request["task"], "compiler": request["compiler"],
             "compiler_sha256": digest(Path(request["compiler"])), "classpath": provenance,
             "sources": [{"path": str(source), "sha256": digest(source)} for source in sources],
             "invocations": [], "fallback": False,
             "compiler_option_differences": {"jvm_target": "Candidate uses its supported classfile versions; configured Gradle target is recorded, not passed as a supported target option."}}
    command = [request["compiler"], *[str(source) for source in sorted(sources)],
               "-d", str(stage), "--module-name", trace["module_name"]]
    if directories:
        command += ["-cp", os.pathsep.join(str(item) for item in directories)]
    print("KORRUST production batch:", *request["sources"], flush=True)
    result = subprocess.run(command, capture_output=True, text=True)
    print(result.stdout, end="", flush=True)
    print(result.stderr, end="", file=sys.stderr, flush=True)
    trace["invocations"].append({"sources": request["sources"], "command": command,
                                 "returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
    write_json(Path(request["trace"]), trace)
    if result.returncode:
        return result.returncode
    publish_stage(stage, destination, manifest)
    trace["published_files_sha256"] = publication_files(destination)
    write_json(Path(request["trace"]), trace)
    return 0


def sources(work, module=MODULE):
    return sorted(path for path in (work / module / "src").rglob("*")
                  if path.suffix in {".kt", ".java"} and path.is_file())


def mapped_model(value, original, snapshot):
    if isinstance(value, dict):
        return {key: mapped_model(item, original, snapshot) for key, item in value.items()}
    if isinstance(value, list):
        return [mapped_model(item, original, snapshot) for item in value]
    if isinstance(value, str):
        try:
            return str(snapshot / Path(value).relative_to(original))
        except ValueError:
            pass
    return value


def source_inventory(work, modules):
    return {str(path.relative_to(work)): digest(path)
            for module in modules for path in sources(work, module)}


def module_inventory(inventory, module):
    selected = {path: value for path, value in inventory.items() if path.startswith(module + "/")}
    return {
        "source_files_sha256": selected,
        "production_kotlin_sources": sorted(path for path in selected if "/src/main/" in path and path.endswith(".kt")),
        "production_java_sources": sorted(path for path in selected if "/src/main/" in path and path.endswith(".java")),
    }


def build(args):
    if "Alkitab" in args.modules and not getattr(args, "capture_only", False):
        raise ValueError("Alkitab is available only with --capture-only; complete app builds are outside this driver")
    if len(set(args.modules)) != len(args.modules):
        raise ValueError("Select each module once")
    is_capture_only = getattr(args, "capture_only", False)
    legacy = args.modules == [MODULE] and not is_capture_only
    default_work = "androidbible-module" if legacy else "androidbible-" + "-".join(name.lower() for name in args.modules)
    work = (args.work_dir or ROOT / "target" / default_work).resolve()
    if work == SOURCE or SOURCE in work.parents or not work.is_relative_to(ROOT / "target"):
        raise ValueError("Disposable build directory must be under project target/, outside original source snapshot")
    artifact_root = (args.artifact_dir or work.parent / (work.name + "-artifacts")).resolve()
    if not artifact_root.is_relative_to(ROOT / "target") or any(
        artifact_root.is_relative_to(work / module / "build") for module in args.modules
    ):
        raise ValueError("Build artifacts must be under project target/, outside selected module build directories")
    artifacts = artifact_root / ("capture" if is_capture_only else args.mode)
    result_path = args.result or (ROOT / "target/androidbible-module-build.json" if legacy else artifacts / "build.json")
    if not work.exists():
        shutil.copytree(SOURCE, work, symlinks=True,
                        ignore=shutil.ignore_patterns(".git", ".gradle", "build", "local.properties"))
    if args.sdk:
        (work / "local.properties").write_text(f"sdk.dir={args.sdk.resolve()}\n")
    source_hashes = source_inventory(work, args.modules)
    original_hashes = source_inventory(SOURCE, args.modules)
    if source_hashes != original_hashes:
        raise ValueError("Disposable selected module sources differ from unchanged benchmark snapshot")
    artifacts.mkdir(parents=True, exist_ok=True)
    init = artifacts / "adapter.init.gradle"
    init.write_text(INIT_SCRIPT)
    module_artifacts = {
        module: artifacts if legacy else artifacts / "modules" / module for module in args.modules
    }
    for directory in module_artifacts.values():
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "gradle-model.json").unlink(missing_ok=True)
        if args.mode == "korrust":
            for name in ["compileDebugKotlin", "compileReleaseKotlin"]:
                (directory / f"{name}.json").unlink(missing_ok=True)
                (directory / f"{name}-request.json").unlink(missing_ok=True)
    compiler = args.compiler.resolve()
    environment = os.environ.copy()
    environment.update({
        "KORRUST_BUILD_MODE": args.mode,
        "KORRUST_CAPTURE_ONLY": "1" if is_capture_only else "0",
        "KORRUST_BUILD_MODULES": json.dumps(args.modules),
        "KORRUST_BUILD_MODULE_ARTIFACTS": json.dumps({module: str(path) for module, path in module_artifacts.items()}),
        "KORRUST_BUILD_COMPILER": str(compiler),
        "KORRUST_BUILD_PYTHON": sys.executable,
        "KORRUST_BUILD_TOOL": str(Path(__file__).resolve()),
        "KORRUST_BUILD_ARTIFACTS": str(artifacts),
        "KORRUST_BUILD_MODEL": str(module_artifacts[args.modules[0]] / "gradle-model.json"),
    })
    gradle = str(args.gradle.resolve()) if args.gradle else str(work / "gradlew")
    task_groups = [
        [f":{module}:clean" for module in args.modules],
        [f":{module}:bundleReleaseAar" for module in args.modules],
        [f":{module}:testDebugUnitTest" for module in args.modules],
        [f":{module}:korrustBuildModel" for module in args.modules],
    ]
    command = [gradle, "--init-script", str(init), "--no-configuration-cache", "--no-build-cache",
               "--max-workers=4", "--console=plain", "--stacktrace",
               *[task for group in task_groups for task in group]]
    if is_capture_only:
        command = command[:command.index(task_groups[0][0])] + [f":{module}:korrustCaptureModel" for module in args.modules]
    log = artifacts / "gradle.log"
    with log.open("w") as output:
        process = subprocess.run(command, cwd=work, env=environment, stdout=output, stderr=subprocess.STDOUT)
    print("\n".join(log.read_text(errors="replace").splitlines()[-60:]), flush=True)
    result = {"schema_version": 2, "kind": "androidbible_build",
              "created_at": datetime.now(timezone.utc).isoformat(), "mode": args.mode, "status": "passed" if process.returncode == 0 else "failed",
              "selected_modules": args.modules, "modules": {},
              "command": command, "returncode": process.returncode, "log": str(log),
              "work_dir": str(work), "source_files_sha256": source_hashes,
              "production_kotlin_sources": sorted(path for path in source_hashes if "/src/main/" in path and path.endswith(".kt")),
              "production_java_sources": sorted(path for path in source_hashes if "/src/main/" in path and path.endswith(".java")),
              "production_kotlin_compiler": "not_run: input capture only" if is_capture_only else "korrust" if args.mode == "korrust" else "official Kotlin Gradle plugin",
              "capture_only": is_capture_only,
              "production_kotlin_fallback": False,
              "compiler": str(compiler) if args.mode == "korrust" else "official Kotlin Gradle plugin",
              "compiler_sha256": digest(compiler) if args.mode == "korrust" and not is_capture_only else None,
              "adapter_sha256": digest(init), "tool_sha256": digest(Path(__file__))}
    snapshot_paths = {module: (work / module / "build", module_artifacts[module] / "module-build")
                      for module in args.modules}
    models = {}
    for module, directory in module_artifacts.items():
        model_path = directory / "gradle-model.json"
        if model_path.exists():
            original, snapshot = snapshot_paths[module]
            shutil.rmtree(snapshot, ignore_errors=True)
            if original.exists():
                shutil.copytree(original, snapshot)
            models[module] = json.loads(model_path.read_text())
    version_checks = {}
    for module, directory in module_artifacts.items():
        entry = module_inventory(source_hashes, module)
        if module in models:
            model = models[module]
            for original, snapshot in snapshot_paths.values():
                model = mapped_model(model, original, snapshot)
            entry.update(model)
            entry["resolved_inputs"] = resolved_inputs(model)
            if entry["resolved_inputs"]["friend_context_failures"]:
                entry.setdefault("setup_failures", []).extend(entry["resolved_inputs"]["friend_context_failures"])
            entry["resolved_kotlin_compiler_files_sha256"] = {
                path: digest(Path(path)) for path in entry["resolved_kotlin_compiler_classpath"]
            }
            compiler_classpath = tuple(entry["resolved_kotlin_compiler_classpath"])
            if compiler_classpath not in version_checks:
                java = str(Path(environment["JAVA_HOME"]) / "bin/java") if "JAVA_HOME" in environment else "java"
                version_command = [java, "-cp", os.pathsep.join(compiler_classpath),
                                   "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler", "-version", "-no-stdlib", "-no-reflect"]
                version = subprocess.run(version_command, capture_output=True, text=True, env=environment)
                version_checks[compiler_classpath] = {
                    "command": version_command, "returncode": version.returncode,
                    "stdout": version.stdout, "stderr": version.stderr,
                }
            entry["resolved_kotlin_compiler_version_check"] = version_checks[compiler_classpath]
            if "test_results" in entry:
                entry["test_results"] = [str(path) for path in Path(entry["test_results"]).glob("TEST-*.xml")]
        if args.mode == "korrust" and not is_capture_only:
            entry["production_compile_traces"] = [str(directory / f"{name}.json") for name in
                                                  ["compileDebugKotlin", "compileReleaseKotlin"]]
        after = source_inventory(work, [module])
        entry["source_files_sha256_after_build"] = after
        entry["sources_unchanged"] = after == source_inventory(SOURCE, [module])
        entry["production_kotlin_compiler"] = result["production_kotlin_compiler"]
        entry["production_kotlin_fallback"] = False
        result["modules"][module] = entry
    missing_inputs = {module: entry["resolved_inputs"]["missing_required_inputs"]
                      for module, entry in result["modules"].items()
                      if entry.get("resolved_inputs", {}).get("missing_required_inputs")}
    if missing_inputs:
        result.update(status="failed", failure_category="setup_failure", missing_required_inputs=missing_inputs)
    capture_failures = {module: entry.get("setup_failures", []) for module, entry in result["modules"].items()
                        if entry.get("setup_failures")}
    if capture_failures:
        result.update(status="failed", failure_category="setup_failure", setup_failures=capture_failures)
    if is_capture_only and len(models) != len(args.modules):
        result.update(status="failed", failure_category="setup_failure", model_capture_error="Selected module model was not emitted")
    if process.returncode:
        result["failure_category"] = failure_category(log.read_text(errors="replace"), bool(models))
    if legacy:
        result.update(result["modules"][MODULE])
    else:
        complete_models = [entry for entry in result["modules"].values() if "resolved_kotlin_compiler_classpath" in entry]
        if complete_models:
            for key in ["resolved_kotlin_compiler_version", "resolved_kotlin_compiler_classpath",
                        "resolved_kotlin_compiler_files_sha256", "resolved_kotlin_compiler_version_check"]:
                result[key] = complete_models[0][key]
        if args.mode == "korrust" and not is_capture_only:
            result["production_compile_traces"] = [path for entry in result["modules"].values()
                                                   for path in entry["production_compile_traces"]]
    result["source_files_sha256_after_build"] = source_inventory(work, args.modules)
    result["sources_unchanged"] = result["source_files_sha256_after_build"] == original_hashes
    if not result["sources_unchanged"]:
        result["status"] = "failed"
        result["source_integrity_error"] = "Selected module sources changed during build"
    write_json(result_path.resolve(), result)
    print(f"Build evidence: {result_path.resolve()}", flush=True)
    return process.returncode or (0 if result["status"] == "passed" else 1)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compile-request", type=Path)
    parser.add_argument("--capture-only", action="store_true", help="Capture selected inputs without compiling production sources; Alkitab selects plain variants")
    parser.add_argument("--mode", choices=["official", "korrust"], default="official")
    parser.add_argument("--modules", nargs="+", choices=["AlkitabModel", "AlkitabIo", "AlkitabYes2", "Alkitab"], default=[MODULE])
    parser.add_argument("--work-dir", type=Path)
    parser.add_argument("--artifact-dir", type=Path)
    parser.add_argument("--result", type=Path)
    parser.add_argument("--gradle", type=Path)
    parser.add_argument("--sdk", type=Path)
    parser.add_argument("--compiler", type=Path, default=ROOT / "target/release/korrust")
    args = parser.parse_args()
    return compile_request(args.compile_request) if args.compile_request else build(args)


if __name__ == "__main__":
    raise SystemExit(main())
