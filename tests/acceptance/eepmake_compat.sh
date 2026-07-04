#!/usr/bin/env bash
#
# Acceptance check: ehatrom's output vs the reference Raspberry Pi tools.
#
# This is intentionally NOT part of CI — it needs the external `eepmake`/`eepdump`
# binaries from raspberrypi/utils (eeptools). Run it locally on a machine that
# has them installed:
#
#     make acceptance
#     # or:
#     tests/acceptance/eepmake_compat.sh
#
# Override tool locations if they are not on PATH:
#
#     EEPMAKE=/path/to/eepmake EEPDUMP=/path/to/eepdump tests/acceptance/eepmake_compat.sh
#
# It checks two things:
#   1. `eepdump` can parse the image ehatrom produced (structural compatibility);
#   2. the image is byte-for-byte identical to what `eepmake` produces from the
#      same settings file (bit-exact compatibility).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
settings="$here/sample_settings.txt"

EEPMAKE="${EEPMAKE:-eepmake}"
EEPDUMP="${EEPDUMP:-eepdump}"

if ! command -v "$EEPMAKE" >/dev/null 2>&1; then
    echo "error: '$EEPMAKE' not found." >&2
    echo "  Install the reference tools: https://github.com/raspberrypi/utils (eeptools)" >&2
    echo "  or set EEPMAKE=/path/to/eepmake." >&2
    exit 2
fi
if ! command -v "$EEPDUMP" >/dev/null 2>&1; then
    echo "error: '$EEPDUMP' not found (set EEPDUMP=/path/to/eepdump)." >&2
    exit 2
fi

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

ours="$workdir/ehatrom.eep"
ref="$workdir/eepmake.eep"

echo "==> building ehatrom (release)"
( cd "$root" && cargo build --release --quiet )
ehatrom="$root/target/release/ehatrom"

echo "==> ehatrom make -> $ours"
"$ehatrom" make "$settings" "$ours"

echo "==> eepmake -> $ref"
"$EEPMAKE" "$settings" "$ref"

echo "==> eepdump parses ehatrom's image"
"$EEPDUMP" "$ours"

echo "==> byte comparison"
if cmp -s "$ours" "$ref"; then
    echo "PASS: ehatrom output is byte-for-byte identical to eepmake"
    exit 0
fi

echo "FAIL: images differ" >&2
echo "-- first differing bytes (offset  ehatrom  eepmake) --" >&2
cmp -l "$ours" "$ref" | head -40 >&2 || true
if command -v xxd >/dev/null 2>&1; then
    echo "-- ehatrom --" >&2; xxd "$ours" >&2
    echo "-- eepmake --" >&2; xxd "$ref" >&2
fi
exit 1
