#!/usr/bin/env bash
# Load the extension with the distribution's sqlite3 and embed one string.
# Runs inside a plain container, e.g. ubuntu:22.04 (glibc 2.35).
set -euo pipefail
so="${1:?usage: smoke-load-extension.sh <libragite.so>}"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq >/dev/null
apt-get install -y -qq sqlite3 >/dev/null
# Capture first: under pipefail, `ldd --version | head -n 1` dies with SIGPIPE
# (141) whenever head exits before ldd has written all its lines.
ldd_out=$(ldd --version)
printf '%s\n' "${ldd_out%%$'\n'*}"
out=$(sqlite3 :memory: ".load $so" \
  "SELECT technical_id FROM ragite_default_model();" \
  "SELECT length(ragite_embed_model((SELECT technical_id FROM ragite_default_model()), 'hello', 'passage'));")
echo "$out"
[[ "$out" == *potion-retrieval-32M@* ]] || { echo "default model missing" >&2; exit 1; }
[[ $'\n'"$out"$'\n' == *$'\n'2048$'\n'* ]] || { echo "embedding length is not 2048" >&2; exit 1; }
