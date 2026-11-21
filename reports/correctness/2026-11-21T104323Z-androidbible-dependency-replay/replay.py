#!/usr/bin/env python3
"""Rebuild genuine AndroidBible dependencies; never modify the pinned source snapshot."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent


def digest(path, algorithm='sha256'):
    h = hashlib.new(algorithm)
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def download(spec, destination):
    algorithm = 'sha256' if 'sha256' in spec else 'sha1'
    if destination.is_file() and digest(destination, algorithm) == spec[algorithm]:
        return
    temporary = destination.with_suffix(destination.suffix + '.download')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(spec['url'], timeout=120) as response, temporary.open('wb') as output:
        shutil.copyfileobj(response, output, 1024 * 1024)
    if digest(temporary, algorithm) != spec[algorithm]:
        raise ValueError('Download checksum mismatch: ' + spec['url'])
    temporary.replace(destination)


def extract(archive, destination):
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive) as contents:
        for entry in contents.infolist():
            target = (destination / entry.filename).resolve()
            if not target.is_relative_to(destination.resolve()):
                raise ValueError('Unsafe archive member: ' + entry.filename)
            contents.extract(entry, destination)
            mode = (entry.external_attr >> 16) & 0o777
            if mode and not entry.is_dir():
                target.chmod(mode)


def run(command, environment, cwd, log, input_text=None):
    with log.open('w') as output:
        result = subprocess.run(command, env=environment, cwd=cwd, input=input_text,
                                text=True, stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        print(log.read_text(errors='replace')[-12000:], file=sys.stderr)
        raise RuntimeError('Command failed; see ' + str(log))


def check_sources(repo, project):
    expected = json.loads((HERE / 'observed-production/source-sha256.json').read_text())
    source = repo / 'benchmarks/sources/androidbible'
    for relative, expected_hash in expected.items():
        for root in (source, project):
            if digest(root / relative) != expected_hash:
                raise ValueError('Pinned source/build file changed: ' + str(root / relative))


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo-root', type=Path, default=HERE.parents[2])
    parser.add_argument('--work-dir', type=Path,
                        default=Path(tempfile.gettempdir()) / 'korrust-androidbible-dependency-replay')
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--sdk', type=Path)
    parser.add_argument('--trust-store', type=Path)
    parser.add_argument('--runtime', action='store_true', help='Also resolve actual Robolectric runtime inputs')
    parser.add_argument('--prepare-only', action='store_true', help='Validate inputs and prepare tools without compilation')
    return parser.parse_args()


def main():
    args = arguments()
    repo, work = args.repo_root.resolve(), args.work_dir.resolve()
    if sys.platform != 'linux':
        raise RuntimeError('This receipt pins Linux Android command-line tools and NDK')
    if args.jdk:
        jdk = args.jdk.resolve()
    elif os.environ.get('JAVA_HOME'):
        jdk = Path(os.environ['JAVA_HOME']).resolve()
    elif shutil.which('javac'):
        jdk = Path(shutil.which('javac')).resolve().parents[1]
    else:
        raise RuntimeError('Install a full JDK 21 and pass --jdk or JAVA_HOME')
    version = subprocess.check_output([str(jdk / 'bin/javac'), '-version'], stderr=subprocess.STDOUT, text=True)
    if not version.startswith('javac 21.'):
        raise RuntimeError('A full JDK 21 is required: ' + version.strip())
    sdk = args.sdk.resolve() if args.sdk else work / 'sdk'
    artifacts, project = work / 'replay-artifacts', work / 'project'
    artifacts.mkdir(parents=True, exist_ok=True)
    if project.is_relative_to(repo / 'benchmarks/sources') or project == repo:
        raise ValueError('Disposable project must be outside the pinned source snapshot')
    if not project.exists():
        shutil.copytree(repo / 'benchmarks/sources/androidbible', project,
                        ignore=shutil.ignore_patterns('.git', '.gradle', 'build', 'local.properties'))
    check_sources(repo, project)
    pins = json.loads((HERE / 'downloads.json').read_text())
    downloads = work / 'downloads'
    downloads.mkdir(exist_ok=True)
    gradle = work / ('gradle-' + pins['gradle']['version']) / 'bin/gradle'
    if not gradle.is_file():
        archive = downloads / 'gradle.zip'
        download(pins['gradle'], archive)
        extract(archive, work)
    manager = sdk / 'cmdline-tools/latest/bin/sdkmanager'
    if not manager.is_file():
        archive = downloads / 'command-line-tools.zip'
        download(pins['command_line_tools'], archive)
        unpack = work / 'command-line-tools-unpack'
        extract(archive, unpack)
        manager.parent.parent.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(unpack / 'cmdline-tools'), str(manager.parent.parent))
    environment = os.environ.copy()
    environment.update(JAVA_HOME=str(jdk), ANDROID_HOME=str(sdk),
                       GRADLE_USER_HOME=str(work / 'gradle-home'),
                       GIT_CEILING_DIRECTORIES=str(work),
                       VERSION_CODE=pins['version_code'], VERSION_STAGE=pins['version_stage'])
    environment['PATH'] = str(jdk / 'bin') + os.pathsep + environment.get('PATH', '')
    properties = ['org.gradle.workers.max=2', 'org.gradle.jvmargs=-Xmx1g -Dfile.encoding=UTF-8',
                  'kotlin.daemon.jvmargs=-Xmx2g', 'org.gradle.java.installations.paths=' + str(jdk),
                  'org.gradle.java.installations.auto-download=false']
    proxy = urllib.parse.urlparse(environment.get('HTTPS_PROXY') or environment.get('https_proxy') or '')
    if proxy.hostname:
        for protocol in ('http', 'https'):
            properties.extend(['systemProp.' + protocol + '.proxyHost=' + proxy.hostname,
                               'systemProp.' + protocol + '.proxyPort=' + str(proxy.port or 80)])
    trust_store = args.trust_store
    if not trust_store and Path('/etc/ssl/certs/java/cacerts').is_file():
        trust_store = Path('/etc/ssl/certs/java/cacerts')
    if trust_store:
        properties.append('systemProp.javax.net.ssl.trustStore=' + str(trust_store.resolve()))
    gradle_home = Path(environment['GRADLE_USER_HOME'])
    gradle_home.mkdir(exist_ok=True)
    (gradle_home / 'gradle.properties').write_text('\n'.join(properties) + '\n')
    (project / 'local.properties').write_text('sdk.dir=' + str(sdk) + '\n')
    sdk_java = [str(jdk / 'bin/java'), *['-D' + value.removeprefix('systemProp.') for value in properties if value.startswith('systemProp.')],
                '-Dcom.android.sdklib.toolsdir=' + str(manager.parent.parent),
                '-classpath', str(manager.parent.parent / 'lib/sdkmanager-classpath.jar'),
                'com.android.sdklib.tool.sdkmanager.SdkManagerCli']
    run([*sdk_java, '--sdk_root=' + str(sdk), *pins['sdk_packages']], environment, work,
        artifacts / 'sdk-install.log', 'y\n' * 1000)
    if digest(sdk / 'platforms/android-36/android.jar') != pins['android_jar_sha256']:
        raise ValueError('Installed SDK36 android.jar differs from the captured revision2 artifact')
    if args.prepare_only:
        print(artifacts)
        return
    environment['ANDROIDBIBLE_CAPTURE_OUTPUT'] = str(artifacts / 'official-model.json')
    run([str(gradle), '--console=plain', '--init-script', str(HERE / 'capture.init.gradle'),
         ':Alkitab:captureDeps33'], environment, project, artifacts / 'official-build.log')
    check_sources(repo, project)
    run([sys.executable, str(HERE / 'filter-classpaths.py'), '--repo-root', str(repo),
         '--work-dir', str(work)], environment, work, artifacts / 'filter.log')
    if args.runtime:
        environment['ANDROIDBIBLE_CAPTURE_OUTPUT'] = str(artifacts / 'test-runtime.json')
        run([str(gradle), '--console=plain', '--init-script', str(HERE / 'capture-test.init.gradle'),
             ':Alkitab:captureTestDeps33'], environment, project, artifacts / 'test-deps-build.log')
        libraries = ['AlkitabIntegration', 'AlkitabYes2', 'AlkitabIo', 'AlkitabModel', 'Bintex', 'Snappy']
        run([str(gradle), '--console=plain', *[':' + name + ':bundleLibRuntimeToJarDebug' for name in libraries]],
            environment, project, artifacts / 'test-runtime-jars-build.log')
        offline = work / 'robolectric-sdk28'
        offline.mkdir(exist_ok=True)
        for spec in pins['offline_sdks']:
            download(spec, offline / Path(spec['url']).name)
        model = json.loads((artifacts / 'test-runtime.json').read_text())
        omitted = [p for p in model['classpath'] if '/runtime_app_classes_jar/' in p]
        paths = [Path(p) for p in model['classpath'] if p not in omitted]
        missing = [str(p) for p in paths if not p.is_file()]
        if missing or len(omitted) != 1:
            raise ValueError('Unexpected runtime inputs: ' + repr(missing))
        receipt = {'dependency_classpath': list(map(str, paths)),
                   'files_sha256': {str(p): digest(p) for p in paths},
                   'omitted_original_app_runtime_jar': omitted,
                   'offline_sdk_directory': str(offline),
                   'policy': 'Use target-filtered sibling JAR; original app target bytecode is excluded.'}
        (artifacts / 'test-runtime-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(artifacts)


if __name__ == '__main__':
    main()
