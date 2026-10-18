#!/usr/bin/env bash
set -euo pipefail
for file in models/*/model.safetensors models/*/tokenizer.json; do
  test -s "$file"
  if head -c 40 "$file" | grep -q 'git-lfs'; then
    echo "model file is still a Git LFS pointer: $file (check out with lfs: true / git lfs pull)"
    exit 1
  fi
done
