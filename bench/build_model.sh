#!/usr/bin/env bash
# Build the extension with one candidate Model linked in at build time (REQ-23).
# Usage: bench/build_model.sh <hf-repo> <name> [query-prefix] [passage-prefix]
# Prints the path of the built libragite.so. Only Model2Vec models load today;
# transformer candidates need the candle loader (REQ-22) first.
set -euo pipefail
hf=$1 name=$2 qp=${3:-} pp=${4:-}
dir=bench/.models/$name
if [ ! -f "$dir/model.safetensors" ]; then
  mkdir -p "$dir"
  for f in config.json tokenizer.json model.safetensors; do
    curl -fsSL -o "$dir/$f" "https://huggingface.co/$hf/resolve/main/$f"
  done
fi
rev=$(curl -fsSL "https://huggingface.co/api/models/$hf" | python3 -c 'import json,sys;print(json.load(sys.stdin)["sha"])')
RAGITE_MODEL_PATH="$PWD/$dir" RAGITE_MODEL_NAME="$name" RAGITE_MODEL_REVISION="$rev" \
RAGITE_QUERY_PREFIX="$qp" RAGITE_PASSAGE_PREFIX="$pp" \
  cargo build -q -p ragite-sqlite --release --no-default-features --target-dir "target/bench-$name" >&2
echo "target/bench-$name/release/libragite.so"
