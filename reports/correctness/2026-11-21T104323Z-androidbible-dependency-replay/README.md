These receipts preserve the genuine AndroidBible dependency build and the ZIP I/O comparison. No SDK, JAR, compiler binary, substitute app class, or modified original source is committed here.

On Linux, install Python 3 and a full JDK 21. Run from any directory:

```sh
python3 /path/to/this/evidence/replay.py --repo-root /path/to/korrust --jdk /path/to/jdk21 --work-dir /tmp/androidbible-replay --runtime
```

The replay downloads checksum-pinned official Gradle and Android command-line tools, accepts the Android SDK licenses, installs the recorded SDK/NDK packages, and runs the unchanged project's official Kotlin/Java build. It checks all 660 recorded source/build-file hashes, pins the observed `VERSION_CODE=24671720`, and prevents an enclosing Git checkout from changing the app's source-archive version metadata. Pass `--sdk` to reuse an existing SDK and `--trust-store` when Java needs an approved custom CA store. Network proxy environment variables are honored.

Outputs go to `<work-dir>/replay-artifacts/`. `classpaths.json` maps the five app-backed survey sources to actual Gradle libraries and their filtered sibling JAR. Every retained sibling class byte is unchanged; the current target's source/class family is excluded. Safe Kotlin package metadata is retained. The absent, empty KSP class-output directory is disclosed rather than supplied as a fake dependency.

`--runtime` also emits `test-runtime-receipt.json`, materializes the six genuine library runtime JARs, and verifies the SDK28 and Android14 Robolectric i6 artifacts. Use its runtime libraries with the target-filtered sibling JAR and the chosen compiled wrapper. The original app runtime JAR is excluded. Original app test sources are not compiled by this dependency capture.

The `observed-production/` and `observed-runtime/` directories are immutable records of the original commands, logs, paths, and hashes. New replay receipts describe newly built artifacts; they do not rewrite those observations. JDK patch-level or build-container differences can affect rebuilt class bytes.

The archive correction is a buffered reader with unchanged ZIP validation and lookup policies. The observed comparison used the same debug build profile and byte-identical source/dependency inputs. Text produced byte-identical rejection diagnostics before and after; this measures classpath I/O, not newly supported Kotlin semantics or Android app build speed. Whole-compiler wall times were observed under concurrent workspace load; CPU times and raw syscall counts are recorded separately.

With Rust/Cargo installed, replay the same-artifact comparison (and syscall counts when `strace` is available):

```sh
python3 /path/to/this/evidence/replay-archive-io.py --artifact /tmp/androidbible-replay/sdk/platforms/android-36/android.jar --work-dir /tmp/archive-io-replay
```

`archive-io/archive-reader.patch` applies the isolated correction to baseline commit `798f7b410`. The archived compiler commands and input inventories record the original real-source probes; the archive unit-test and all-target Clippy logs accompany them.
