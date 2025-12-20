#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_SRC="$ROOT_DIR/crates/agtrace-cli/src"

if ! command -v rg >/dev/null 2>&1; then
  echo "Error: ripgrep (rg) required" >&2
  exit 2
fi

JSON_OUT=""
if [[ "${1:-}" == "--json" ]]; then
  JSON_OUT="${2:-}"
  [[ -z "$JSON_OUT" ]] && { echo "Error: --json requires path" >&2; exit 2; }
fi

HANDLERS="$CLI_SRC/handlers"
PRESENTATION="$CLI_SRC/presentation"

# I/O 例外（本質的に許容する handlers）
IO_EXCEPTIONS=(
  "$HANDLERS/doctor_inspect.rs"
  "$HANDLERS/session_show.rs"
)

# ---------- helpers ----------
rg_lines() { rg --no-heading --line-number -S "$1" "$2" || true; }
count_lines() { [[ -z "${1:-}" ]] && echo 0 || echo "$1" | sed '/^\s*$/d' | wc -l | tr -d ' '; }
json_escape() { printf '%s' "${1:-}" | sed 's/\\/\\\\/g;s/"/\\"/g;s/\r/\\r/g;s/\n/\\n/g;s/\t/\\t/g'; }

# ---------- Rule 1: handlers direct print ----------
R1_NAME="handlers_no_direct_print"
R1_MATCH="$(rg_lines '(^|[^[:alnum:]_])(println!|eprintln!|dbg!)\s*\(' "$HANDLERS")"

# ---------- Rule 2: handlers deep presentation deps ----------
R2_NAME="handlers_no_deep_presentation"
R2_MATCH="$(rg_lines 'crate::presentation::(views|formatters|renderers::(console|tui|refresh|backend))\b' "$HANDLERS")"

# ---------- Rule 3: presentation -> handlers ----------
R3_NAME="presentation_no_handlers"
R3_MATCH="$(rg_lines 'crate::handlers\b' "$PRESENTATION")"

# ---------- Rule 4: presentation should not touch domain/infra ----------
R4_NAME="presentation_no_domain_infra"
R4_MATCH="$(rg_lines 'agtrace_(index|runtime|providers)|walkdir::|std::fs\b|File::open\b' "$PRESENTATION")"

# ---------- Rule 5: handlers should not use formatters (strict) ----------
R5_NAME="handlers_no_formatters"
R5_MATCH="$(rg_lines 'crate::presentation::formatters\b' "$HANDLERS")"

# ---------- Rule 6: handlers raw IO (with exceptions) ----------
R6_NAME="handlers_no_raw_io"
R6_RAW="$(rg_lines 'std::fs::|std::fs\b|File::open\b|read_to_string\b|fs::read_to_string\b|std::fs::metadata\b' "$HANDLERS")"

R6_MATCH=""
if [[ -n "$R6_RAW" ]]; then
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    skip=false
    for ex in "${IO_EXCEPTIONS[@]}"; do
      if [[ "$line" == "$ex:"* ]]; then
        skip=true
        break
      fi
    done
    if [[ "$skip" == false ]]; then
      R6_MATCH+="$line"$'\n'
    fi
  done <<<"$R6_RAW"
fi

# ---------- Rule 7: handlers file too large (warning-grade but counted) ----------
R7_NAME="handlers_file_too_large"
R7_MATCH=""
while IFS= read -r f; do
  loc=$(wc -l <"$f" | tr -d ' ')
  if [[ "$loc" -gt 250 ]]; then
    R7_MATCH+="$f:$loc"$'\n'
  fi
done < <(find "$HANDLERS" -name "*.rs" -print)

# ---------- Report (stable order) ----------
declare -A RULES
RULES[$R1_NAME]="$R1_MATCH"
RULES[$R2_NAME]="$R2_MATCH"
RULES[$R3_NAME]="$R3_MATCH"
RULES[$R4_NAME]="$R4_MATCH"
RULES[$R5_NAME]="$R5_MATCH"
RULES[$R6_NAME]="$R6_MATCH"
RULES[$R7_NAME]="$R7_MATCH"

ORDER=(
  "$R1_NAME"
  "$R6_NAME"
  "$R5_NAME"
  "$R2_NAME"
  "$R4_NAME"
  "$R3_NAME"
  "$R7_NAME"
)

TOTAL=0
echo "Layer Boundary Report v2.1"
echo "Root: $ROOT_DIR"
echo "----------------------------------------"

for name in "${ORDER[@]}"; do
  matches="${RULES[$name]}"
  c=$(count_lines "$matches")
  TOTAL=$((TOTAL + c))
  echo "$name : $c"
  if [[ "$c" -gt 0 ]]; then
    echo "$matches" | sed '/^\s*$/d' | sed 's/^/  - /'
  fi
  echo
done

echo "----------------------------------------"
echo "Total violations: $TOTAL"

# ---------- JSON ----------
if [[ -n "$JSON_OUT" ]]; then
  mkdir -p "$(dirname "$JSON_OUT")"
  {
    echo "{"
    echo "  \"root\": \"$(json_escape "$ROOT_DIR")\","
    echo "  \"total\": $TOTAL,"
    echo "  \"rules\": {"
    for i in "${!ORDER[@]}"; do
      name="${ORDER[$i]}"
      c=$(count_lines "${RULES[$name]}")
      comma=","
      [[ "$i" -eq $((${#ORDER[@]} - 1)) ]] && comma=""
      echo "    \"$(json_escape "$name")\": $c$comma"
    done
    echo "  }"
    echo "}"
  } >"$JSON_OUT"
  echo "JSON written to $JSON_OUT"
fi

[[ "$TOTAL" -gt 0 ]] && exit 1
exit 0
