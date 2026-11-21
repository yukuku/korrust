#!/bin/sh
set -eu
replay_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$replay_dir/replay.py" "$@"
