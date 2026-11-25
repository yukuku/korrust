#!/usr/bin/env bash
set -euo pipefail
cd /workspace/korrust
source /workspace/korrust-setup/env.sh
export GRADLE_USER_HOME=/workspace/work/deps33/gradle-home ANDROID_HOME=/workspace/work/deps33/sdk GIT_CEILING_DIRECTORIES=/workspace/korrust/target
candidate=$1
out=$2
python3 tools/build_androidbible.py --mode korrust --modules AlkitabModel AlkitabIo --work-dir "target/$out-project" --artifact-dir "target/$out-artifacts" --result "target/$out-build.json" --compiler "$candidate" --gradle /workspace/work/deps33/gradle-9.7.1/bin/gradle --sdk "$ANDROID_HOME" 2>&1 | tail -60
python3 tools/check_androidbible_model_io_consumers.py --official-json /workspace/work/modelio25/official-build.json --candidate-json "target/$out-build.json" --skip-html
