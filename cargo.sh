#!/bin/sh

# This script manages Cargo operations
# while keeping the artifact directory within the build tree
# instead of the source tree

set -e

SCRIPT_PATH="$(realpath "$0")"
SOURCE_DIR="$(dirname "$SCRIPT_PATH")"

CARGO_TARGET_DIR="$(pwd)"
export CARGO_TARGET_DIR

cd "$SOURCE_DIR"

# the 'run" command takes arguments at the end,
# so --manifest-path must not be last
CMD="$1"
shift
cargo "$CMD" --manifest-path "$CARGO_TARGET_DIR"/Cargo.toml "$@"

