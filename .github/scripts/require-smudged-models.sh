#!/usr/bin/env bash
set -euo pipefail
for file in models/*/model.safetensors models/*/tokenizer.json; do
  test -s "$file"
  if head -c 40 "$file" | grep -q 'git-lfs'; then
    echo "model file is still a Git LFS pointer: $file (restore the LFS cache or run git lfs pull, then git lfs checkout)"
    exit 1
  fi
done
