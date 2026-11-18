#!/usr/bin/env bash
# Usage: ux-climb/capture.sh <round>   (wrapper; the work is in capture.mjs)
exec node "$(dirname "$0")/capture.mjs" "$@"
