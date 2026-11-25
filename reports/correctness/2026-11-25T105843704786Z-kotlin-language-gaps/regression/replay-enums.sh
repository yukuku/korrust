#!/usr/bin/env bash
set -eu
candidate=$1
directory=$2
stdlib=references/kotlinc/lib/kotlin-stdlib.jar
sources=tests/operator_override_regression
mkdir -p "$directory"
references/kotlinc/bin/kotlinc "$sources/Enums.kt" -jvm-target 1.8 -cp "$stdlib" -d "$directory/official"
"$candidate" "$sources/Enums.kt" -cp "$stdlib" -d "$directory/candidate"
javac -cp "$directory/official:$stdlib" -d "$directory/driver" "$sources/EnumCalls.java"
for provider in official candidate; do
  java -Xverify:all -cp "$directory/driver:$directory/$provider:$stdlib" EnumCalls
done
