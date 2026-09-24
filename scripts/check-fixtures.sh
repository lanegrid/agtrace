#!/usr/bin/env bash
# Sanitization guard for synthetic test fixtures.
#
# Fixtures must be synthetic: no real user paths, e-mails, tokens or ids from real logs.
# Fails on:
#   - absolute home paths (/Users/..., /home/...)
#   - e-mail addresses (except @example.com / @example.org)
#   - sk- / cse_ / toolu_ values that are not marked synthetic (e.g. toolu_synthetic_01)
#   - files larger than 64 KiB
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="${1:-$ROOT/crates/agtrace-testing/fixtures}"

if [[ ! -d "$FIXTURES" ]]; then
  echo "fixtures:check: no fixture directory at $FIXTURES" >&2
  exit 1
fi

status=0
report() {
  echo "fixtures:check: $1" >&2
  status=1
}

while IFS= read -r -d '' file; do
  rel="${file#"$ROOT"/}"
  size=$(wc -c <"$file" | tr -d ' ')
  if (( size > 65536 )); then
    report "$rel is larger than 64 KiB ($size bytes)"
  fi
  if grep -nE '/Users/|/home/[A-Za-z0-9._-]+' "$file" >/dev/null; then
    report "$rel contains an absolute home path: $(grep -noE '/Users/[^"/ ]*|/home/[A-Za-z0-9._-]+' "$file" | head -1)"
  fi
  if grep -noE '[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}' "$file" \
    | grep -vE '@example\.(com|org)$' >/dev/null; then
    report "$rel contains an e-mail address"
  fi
  if grep -noE '(sk-|cse_|toolu_)[A-Za-z0-9_-]+' "$file" | grep -vE ':(sk-|cse_|toolu_)synthetic' >/dev/null; then
    report "$rel contains a non-synthetic token/id: $(grep -noE '(sk-|cse_|toolu_)[A-Za-z0-9_-]+' "$file" | grep -vE ':(sk-|cse_|toolu_)synthetic' | head -1)"
  fi
done < <(find "$FIXTURES" -type f -print0)

if (( status == 0 )); then
  echo "fixtures:check: OK ($(find "$FIXTURES" -type f | wc -l | tr -d ' ') files)"
fi
exit "$status"
