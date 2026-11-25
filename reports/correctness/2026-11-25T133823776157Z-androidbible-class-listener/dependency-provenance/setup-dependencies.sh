#!/bin/bash
set -euo pipefail
# Rebuild only the unchanged pinned AndroidBible snapshot's dependency artifacts.
deps_root=/workspace/work/deps33
deps_project="$deps_root/project"
deps_sdk="$deps_root/sdk"
mkdir -p "$deps_root"
if [[ ! -d "$deps_project" ]]; then
  cp -a /workspace/korrust/benchmarks/sources/androidbible "$deps_project"
fi
if [[ ! -x "$deps_root/gradle-9.7.1/bin/gradle" ]]; then
  curl -fL --retry 2 https://services.gradle.org/distributions/gradle-9.7.1-bin.zip -o "$deps_root/gradle.zip"
  curl -fL --retry 2 https://services.gradle.org/distributions/gradle-9.7.1-bin.zip.sha256 -o "$deps_root/gradle.zip.sha256"
  python3 - <<'PY'
import hashlib
from pathlib import Path
w=Path('/workspace/work/deps33')
assert hashlib.sha256((w/'gradle.zip').read_bytes()).hexdigest()==(w/'gradle.zip.sha256').read_text().strip()
PY
  unzip -q "$deps_root/gradle.zip" -d "$deps_root"
  rm "$deps_root/gradle.zip"
fi
if [[ ! -x "$deps_sdk/cmdline-tools/latest/bin/sdkmanager" ]]; then
  curl -fL --retry 2 https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip -o "$deps_root/cmdtools.zip"
  mkdir -p "$deps_sdk/cmdline-tools"
  unzip -q "$deps_root/cmdtools.zip" -d "$deps_sdk/cmdline-tools"
  mv "$deps_sdk/cmdline-tools/cmdline-tools" "$deps_sdk/cmdline-tools/latest"
  rm "$deps_root/cmdtools.zip"
fi
python3 - <<'PY'
import os,subprocess
from pathlib import Path
from urllib.parse import urlparse
w=Path('/workspace/work/deps33');h=w/'gradle-home';h.mkdir(exist_ok=True)
p=['systemProp.javax.net.ssl.trustStore=/etc/ssl/certs/java/cacerts','org.gradle.workers.max=2','org.gradle.jvmargs=-Xmx1g -Dfile.encoding=UTF-8','kotlin.daemon.jvmargs=-Xmx2g','org.gradle.java.installations.paths=/workspace/korrust-setup/jdk-21','org.gradle.java.installations.auto-download=false']
u=urlparse(os.environ.get('HTTPS_PROXY') or os.environ.get('https_proxy') or '')
if u.hostname:
 for protocol in ['http','https']:
  p += [f'systemProp.{protocol}.proxyHost={u.hostname}',f'systemProp.{protocol}.proxyPort={u.port or 80}']
(h/'gradle.properties').write_text('\n'.join(p)+'\n')
(w/'project/local.properties').write_text('sdk.dir='+str(w/'sdk')+'\n')
subprocess.run([str(w/'sdk/cmdline-tools/latest/bin/sdkmanager'),'--sdk_root='+str(w/'sdk'),'platforms;android-36','build-tools;36.0.0','platform-tools','ndk;28.2.13676358'],input='y\n'*1000,text=True,check=True)
PY
