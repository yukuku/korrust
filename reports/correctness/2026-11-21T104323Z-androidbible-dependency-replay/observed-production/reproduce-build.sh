#!/bin/bash
set -euo pipefail
bash /workspace/work/deps33/setup-dependencies.sh
cd /workspace/work/deps33/project
export GRADLE_USER_HOME=/workspace/work/deps33/gradle-home
export JAVA_HOME=/workspace/korrust-setup/jdk-21
export ANDROID_HOME=/workspace/work/deps33/sdk
/workspace/work/deps33/gradle-9.7.1/bin/gradle --console=plain --init-script /workspace/work/deps33/capture.init.gradle :Alkitab:compilePlainDebugJavaWithJavac :Alkitab:captureDeps33
python3 /workspace/work/deps33/filter_classpaths.py
