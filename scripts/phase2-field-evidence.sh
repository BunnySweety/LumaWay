#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

fps=""
elapsed_seconds=""
calibrate_used="no"
latency_threshold_ms="300"
first_run_threshold_seconds="600"
min_transitions="5"
declare -a pairs=()

usage() {
    cat <<'EOF'
Usage:
  scripts/phase2-field-evidence.sh --fps <fps> --elapsed-seconds <seconds> [options] <screen_frame:light_frame>...

Options:
  --calibrate-used yes|no              Whether calibrate-capture was used in the timed flow. Default: no.
  --latency-threshold-ms <ms>          Visible latency gate. Default: 300.
  --first-run-threshold-seconds <sec>  First-run timing gate. Default: 600.
  --min-transitions <n>                Minimum accepted video transitions. Default: 5.

Example:
  scripts/phase2-field-evidence.sh --fps 120 --elapsed-seconds 420 --calibrate-used no 100:124 220:247 340:369 460:490 580:613

Each frame pair is measured from the first video frame where the screen changes
to the first frame where any target Hue light visibly changes.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fps)
            [[ $# -ge 2 ]] || {
                echo "missing value for --fps" >&2
                exit 2
            }
            fps="$2"
            shift 2
            ;;
        --elapsed-seconds)
            [[ $# -ge 2 ]] || {
                echo "missing value for --elapsed-seconds" >&2
                exit 2
            }
            elapsed_seconds="$2"
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
        --latency-threshold-ms)
            [[ $# -ge 2 ]] || {
                echo "missing value for --latency-threshold-ms" >&2
                exit 2
            }
            latency_threshold_ms="$2"
            shift 2
            ;;
        --first-run-threshold-seconds)
            [[ $# -ge 2 ]] || {
                echo "missing value for --first-run-threshold-seconds" >&2
                exit 2
            }
            first_run_threshold_seconds="$2"
            shift 2
            ;;
        --min-transitions)
            [[ $# -ge 2 ]] || {
                echo "missing value for --min-transitions" >&2
                exit 2
            }
            min_transitions="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            while [[ $# -gt 0 ]]; do
                pairs+=("$1")
                shift
            done
            ;;
        -*)
            echo "unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
        *)
            pairs+=("$1")
            shift
            ;;
    esac
done

if [[ -z "$fps" ]]; then
    echo "missing required --fps" >&2
    usage >&2
    exit 2
fi

if [[ -z "$elapsed_seconds" ]]; then
    echo "missing required --elapsed-seconds" >&2
    usage >&2
    exit 2
fi

set +e
latency_output="$("$script_dir/phase2-latency-summary.sh" \
    --fps "$fps" \
    --threshold-ms "$latency_threshold_ms" \
    --min-transitions "$min_transitions" \
    "${pairs[@]}" 2>&1)"
latency_status=$?

first_run_output="$("$script_dir/phase2-first-run-summary.sh" \
    --elapsed-seconds "$elapsed_seconds" \
    --threshold-seconds "$first_run_threshold_seconds" \
    --calibrate-used "$calibrate_used" 2>&1)"
first_run_status=$?
set -e

if (( latency_status == 0 )); then
    latency_verdict="pass"
else
    latency_verdict="fail"
fi

if (( first_run_status == 0 )); then
    first_run_verdict="pass"
else
    first_run_verdict="fail"
fi

echo "phase2_field_evidence"
printf "latency=%s first_run=%s\n" "$latency_verdict" "$first_run_verdict"
printf "fps=%s elapsed_seconds=%s calibrate_used=%s\n" "$fps" "$elapsed_seconds" "$calibrate_used"
echo
echo "[latency]"
printf "%s\n" "$latency_output"
echo
echo "[first_run]"
printf "%s\n" "$first_run_output"

if (( latency_status == 0 && first_run_status == 0 )); then
    echo "phase2_field_evidence=pass"
    exit 0
fi

echo "phase2_field_evidence=fail"

if (( latency_status == 2 || first_run_status == 2 )); then
    exit 2
fi

exit 1
