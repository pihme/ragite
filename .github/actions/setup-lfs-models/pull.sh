#!/usr/bin/env bash
# Download Git LFS objects on a cache miss. Auth is passed through the
# environment for this process only; nothing is written to .git/config.
# That keeps persist-credentials: false on the jobs that set it.
set -euo pipefail

if ! command -v git-lfs >/dev/null 2>&1 && ! git lfs version >/dev/null 2>&1; then
  echo "git lfs is not installed on this runner" >&2
  exit 1
fi

export GIT_TERMINAL_PROMPT=0

# If actions/checkout already stored credentials, git lfs pull can use them.
if git config --get http.https://github.com/.extraheader >/dev/null 2>&1; then
  git lfs pull
  exit 0
fi

if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "GITHUB_TOKEN is not set; cannot authenticate git lfs pull" >&2
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
chmod 700 "$tmp"

# Same basic-auth header actions/checkout uses. Never print it.
auth=$(printf 'x-access-token:%s' "$GITHUB_TOKEN" | base64 | tr -d '\r\n')
# git-config uses a leading hyphen style that can break on some versions if
# the value is passed on the command line; a private config file is safer.
# The tab before extraHeader is the usual git-config form.
{
  printf '[http "https://github.com/"]\n'
  printf '\textraHeader = AUTHORIZATION: basic %s\n' "$auth"
} >"$tmp/gitconfig"
chmod 600 "$tmp/gitconfig"
unset auth

GIT_CONFIG_GLOBAL="$tmp/gitconfig" git lfs pull
