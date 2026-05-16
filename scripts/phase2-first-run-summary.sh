#!/usr/bin/env bash
set -euo pipefail

threshold_seconds="600"
elapsed_seconds=""
calibrate_used="no"

usage() {
    cat <<'EOF'
Usage:
  scripts/phase2-first-run-summary.sh --elapsed-seconds <seconds> [--threshold-seconds <seconds>] [--calibrate-used yes|no]

Example:
  scripts/phase2-first-run-summary.sh --elapsed-seconds 420 --calibrate-used no

Measure from launching the installed LumaWay app to the first satisfactory
non-black TV/monitor sync. The Phase 2 target is 600 seconds or less without
running calibrate-capture.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --elapsed-seconds)
            [[ $# -ge 2 ]] || {
                echo "missing value for --elapsed-seconds" >&2
                exit 2
            }
            elapsed_seconds="$2"
            shift 2
            ;;
        --threshold-seconds)
            [[ $# -ge 2 ]] || {
                echo "missing value for --threshold-seconds" >&2
                exit 2
            }
            threshold_seconds="$2"
            shift 2
            ;;
        --calibrate-used)
            [[ $# -ge 2 ]] || {
                echo "missing value for --calibrate-used" >&2
                exit 2
            }
            calibrate_used="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

number_re='^[0-9]+([.][0-9]+)?$'

if [[ -z "$elapsed_seconds" || ! "$elapsed_seconds" =~ $number_re ]]; then
    echo "--elapsed-seconds must be a non-negative number" >&2
    exit 2
fi

if [[ ! "$threshold_seconds" =~ $number_re ]]; then
    echo "--threshold-seconds must be a positive number" >&2
    exit 2
fi

awk -v value="$threshold_seconds" 'BEGIN { exit !(value > 0) }' || {
    echo "--threshold-seconds must be greater than zero" >&2
    exit 2
}

case "$calibrate_used" in
    yes|no)
        ;;
    *)
        echo "--calibrate-used must be yes or no" >&2
        exit 2
        ;;
esac

elapsed_ms="$(awk -v seconds="$elapsed_seconds" 'BEGIN { printf "%.0f", seconds * 1000 }')"
threshold_ms="$(awk -v seconds="$threshold_seconds" 'BEGIN { printf "%.0f", seconds * 1000 }')"
timing_result="$(awk -v elapsed="$elapsed_seconds" -v threshold="$threshold_seconds" 'BEGIN { print (elapsed <= threshold) ? "pass" : "fail" }')"

if [[ "$calibrate_used" == "yes" ]]; then
    calibrate_result="fail"
else
    calibrate_result="pass"
fi

printf "elapsed_seconds=%s threshold_seconds=%s elapsed_ms=%s threshold_ms=%s\n" \
    "$elapsed_seconds" "$threshold_seconds" "$elapsed_ms" "$threshold_ms"
printf "timing=%s calibrate_capture=%s\n" "$timing_result" "$calibrate_result"

if [[ "$timing_result" == "fail" || "$calibrate_result" == "fail" ]]; then
    echo "phase2_first_run=fail"
    exit 1
fi

echo "phase2_first_run=pass"
