#!/bin/bash
set -euo pipefail
export GRADLE_USER_HOME=/workspace/work/deps33/gradle-home
export JAVA_HOME=/workspace/korrust-setup/jdk-21
export ANDROID_HOME=/workspace/work/deps33/sdk
cd /workspace/work/deps33/project
/workspace/work/deps33/gradle-9.7.1/bin/gradle --console=plain --init-script /workspace/work/deps33/capture-test.init.gradle :Alkitab:captureTestDeps33
/workspace/work/deps33/gradle-9.7.1/bin/gradle --console=plain :AlkitabIntegration:bundleLibRuntimeToJarDebug :AlkitabYes2:bundleLibRuntimeToJarDebug :AlkitabIo:bundleLibRuntimeToJarDebug :AlkitabModel:bundleLibRuntimeToJarDebug :Bintex:bundleLibRuntimeToJarDebug :Snappy:bundleLibRuntimeToJarDebug
mkdir -p /workspace/work/deps33/robolectric-sdk28
curl -fL --retry 2 -o /workspace/work/deps33/robolectric-sdk28/android-all-instrumented-9-robolectric-4913185-2-i6.jar https://repo.maven.apache.org/maven2/org/robolectric/android-all-instrumented/9-robolectric-4913185-2-i6/android-all-instrumented-9-robolectric-4913185-2-i6.jar
python3 - <<'PY'
import hashlib
from pathlib import Path
p=Path('/workspace/work/deps33/robolectric-sdk28/android-all-instrumented-9-robolectric-4913185-2-i6.jar')
assert hashlib.sha256(p.read_bytes()).hexdigest()=='20c7c3d1dba85bbc96ffdfabf2e3eefbdad455927691e4897096125d09bf58ea'
PY
