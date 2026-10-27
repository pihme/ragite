#!/usr/bin/env bash
# Load the extension with the distribution's sqlite3 and embed one string.
# Runs inside a plain container, e.g. ubuntu:22.04 (glibc 2.35).
set -euo pipefail
so="${1:?usage: smoke-load-extension.sh <libragite.so>}"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq >/dev/null
apt-get install -y -qq sqlite3 >/dev/null
ldd --version | head -n 1
out=$(sqlite3 :memory: ".load $so" \
  "SELECT technical_id FROM ragite_default_model();" \
  "SELECT length(ragite_embed_model((SELECT technical_id FROM ragite_default_model()), 'hello', 'passage'));")
echo "$out"
echo "$out" | grep -q 'potion-retrieval-32M@'
echo "$out" | grep -qx 2048
