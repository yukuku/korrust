import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import zipfile


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
            result
        }
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
                        unit_test_friend_paths: unitKotlin.'getFriendPathsSet$kotlin_gradle_plugin_common'().get().collect { new File(selected.layout.buildDirectory.get().asFile, it).absolutePath }.sort(),
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
'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def resolved_inputs(model):
    paths = {}
    for name, task in model.get("compilation_tasks", {}).items():
        for key in ("sources", "compile_classpath", "compiler_plugin_classpath"):
            for item in task.get(key, []):
                paths.setdefault(item, []).append(name + ":" + key)
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


def unpack_classpath(paths, destination):
    directories = []
    provenance = []
    for item in paths:
        path = Path(item)
        if path.is_dir():
            directories.append(path)
            provenance.append({"path": str(path), "kind": "directory"})
        elif path.is_file() and path.suffix == ".jar":
            checksum = digest(path)
            extracted = destination / checksum
            if not extracted.exists():
                extracted.mkdir(parents=True)
                with zipfile.ZipFile(path) as archive:
                    for member in archive.infolist():
                        if not member.filename.endswith(".class"):
                            continue
                        relative = PurePosixPath(member.filename)
                        if relative.is_absolute() or ".." in relative.parts or "\\" in member.filename:
                            raise ValueError(f"Unsafe classpath archive entry: {path}: {member.filename}")
                        output = extracted.joinpath(*relative.parts)
                        output.parent.mkdir(parents=True, exist_ok=True)
                        output.write_bytes(archive.read(member))
            directories.append(extracted)
            provenance.append({"path": str(path), "kind": "jar", "sha256": checksum,
                               "extracted": str(extracted)})
        else:
            raise ValueError(f"Unsupported or missing resolved compilation classpath entry: {path}")
    return directories, provenance


def compile_request(path):
    request = json.loads(path.read_text())
    work = Path(request["work_dir"])
    module = request.get("module", MODULE)
    expected = sorted(str(p) for p in (work / module / "src/main").rglob("*.kt"))
    if request["sources"] != expected:
        raise ValueError(f"Gradle production Kotlin sources differ from selected module inventory: {request['sources']} != {expected}")
    stage = Path(request["stage"])
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    directories, provenance = unpack_classpath(
        request["classpath"], work / module / "build/tmp/korrust-classpath" / request["task"].rsplit(":", 1)[-1]
    )
    trace = {"module": module, "task": request["task"], "compiler": request["compiler"],
             "compiler_sha256": digest(Path(request["compiler"])),
             "classpath": provenance, "invocations": [], "fallback": False}
    for source in request["sources"]:
        command = [request["compiler"], source, "-d", str(stage)]
        if directories:
            command += ["-cp", os.pathsep.join(str(p) for p in directories)]
        print("KORRUST production source:", source, flush=True)
        result = subprocess.run(command, capture_output=True, text=True)
        print(result.stdout, end="", flush=True)
        print(result.stderr, end="", file=sys.stderr, flush=True)
        trace["invocations"].append({"source": source, "source_sha256": digest(Path(source)),
                                     "command": command, "returncode": result.returncode,
                                     "stdout": result.stdout, "stderr": result.stderr})
        write_json(Path(request["trace"]), trace)
        if result.returncode:
            return result.returncode
    destination = Path(request["destination"])
    shutil.rmtree(destination, ignore_errors=True)
    destination.mkdir(parents=True)
    for child in stage.iterdir():
        if child.name == ".korrust":
            continue
        if child.is_dir():
            shutil.copytree(child, destination / child.name)
        else:
            shutil.copy2(child, destination / child.name)
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
    if len(set(args.modules)) != len(args.modules):
        raise ValueError("Select each module once")
    legacy = args.modules == [MODULE]
    default_work = "androidbible-module" if legacy else "androidbible-" + "-".join(name.lower() for name in args.modules)
    work = (args.work_dir or ROOT / "target" / default_work).resolve()
    if work == SOURCE or SOURCE in work.parents or not work.is_relative_to(ROOT / "target"):
        raise ValueError("Disposable build directory must be under project target/, outside original source snapshot")
    artifact_root = (args.artifact_dir or work.parent / (work.name + "-artifacts")).resolve()
    if not artifact_root.is_relative_to(ROOT / "target") or any(
        artifact_root.is_relative_to(work / module / "build") for module in args.modules
    ):
        raise ValueError("Build artifacts must be under project target/, outside selected module build directories")
    artifacts = artifact_root / args.mode
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
              "production_kotlin_compiler": "korrust" if args.mode == "korrust" else "official Kotlin Gradle plugin",
              "production_kotlin_fallback": False,
              "compiler": str(compiler) if args.mode == "korrust" else "official Kotlin Gradle plugin",
              "compiler_sha256": digest(compiler) if args.mode == "korrust" else None,
              "adapter_sha256": digest(init), "tool_sha256": digest(Path(__file__))}
    snapshot_paths = {module: (work / module / "build", module_artifacts[module] / "module-build")
                      for module in args.modules}
    models = {}
    for module, directory in module_artifacts.items():
        model_path = directory / "gradle-model.json"
        if model_path.exists():
            original, snapshot = snapshot_paths[module]
            shutil.rmtree(snapshot, ignore_errors=True)
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
            entry["test_results"] = [str(path) for path in Path(entry["test_results"]).glob("TEST-*.xml")]
        if args.mode == "korrust":
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
        if args.mode == "korrust":
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
    parser.add_argument("--mode", choices=["official", "korrust"], default="official")
    parser.add_argument("--modules", nargs="+", choices=["AlkitabModel", "AlkitabIo", "AlkitabYes2"], default=[MODULE])
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
