#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

fps=""
elapsed_seconds=""
calibrate_used="no"
silent_black=""
latency_threshold_ms="300"
first_run_threshold_seconds="600"
min_fps="120"
min_transitions="5"
declare -a pairs=()

usage() {
    cat <<'EOF'
Usage:
  scripts/phase2-field-evidence.sh --fps <fps> --elapsed-seconds <seconds> --silent-black yes|no [options] <screen_frame:light_frame>...

Options:
  --calibrate-used yes|no              Whether calibrate-capture was used in the timed flow. Default: no.
  --silent-black yes|no                Whether a non-black pattern stayed black silently. Required.
  --latency-threshold-ms <ms>          Visible latency gate. Default: 300.
  --first-run-threshold-seconds <sec>  First-run timing gate. Default: 600.
  --min-fps <fps>                      Minimum recording frame rate. Default: 120.
  --min-transitions <n>                Minimum accepted video transitions. Default: 5.

Example:
  scripts/phase2-field-evidence.sh --fps 120 --elapsed-seconds 420 --calibrate-used no --silent-black no 100:124 220:247 340:369 460:490 580:613

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
        --silent-black)
            [[ $# -ge 2 ]] || {
                echo "missing value for --silent-black" >&2
                exit 2
            }
            silent_black="$2"
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
        --min-fps)
            [[ $# -ge 2 ]] || {
                echo "missing value for --min-fps" >&2
                exit 2
            }
            min_fps="$2"
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

if [[ -z "$silent_black" ]]; then
    echo "missing required --silent-black" >&2
    usage >&2
    exit 2
fi

number_re='^[0-9]+([.][0-9]+)?$'

if [[ ! "$fps" =~ $number_re ]]; then
    echo "--fps must be a positive number" >&2
    exit 2
fi

if [[ ! "$min_fps" =~ $number_re ]]; then
    echo "--min-fps must be a positive number" >&2
    exit 2
fi

awk -v value="$fps" 'BEGIN { exit !(value > 0) }' || {
    echo "--fps must be greater than zero" >&2
    exit 2
}

awk -v value="$min_fps" 'BEGIN { exit !(value > 0) }' || {
    echo "--min-fps must be greater than zero" >&2
    exit 2
}

case "$silent_black" in
    yes)
        silent_black_status=1
        silent_black_verdict="fail"
        ;;
    no)
        silent_black_status=0
        silent_black_verdict="pass"
        ;;
    *)
        echo "--silent-black must be yes or no" >&2
        exit 2
        ;;
esac

if awk -v fps="$fps" -v min="$min_fps" 'BEGIN { exit !(fps >= min) }'; then
    video_fps_status=0
    video_fps_verdict="pass"
else
    video_fps_status=1
    video_fps_verdict="fail"
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
printf "video_fps=%s latency=%s first_run=%s silent_black=%s\n" "$video_fps_verdict" "$latency_verdict" "$first_run_verdict" "$silent_black_verdict"
printf "fps=%s min_fps=%s elapsed_seconds=%s calibrate_used=%s silent_black_observed=%s\n" "$fps" "$min_fps" "$elapsed_seconds" "$calibrate_used" "$silent_black"
echo
echo "[video_fps]"
printf "fps=%s min_fps=%s result=%s\n" "$fps" "$min_fps" "$video_fps_verdict"
echo
echo "[latency]"
printf "%s\n" "$latency_output"
echo
echo "[first_run]"
printf "%s\n" "$first_run_output"
echo
echo "[silent_black]"
printf "observed=%s result=%s\n" "$silent_black" "$silent_black_verdict"

if (( video_fps_status == 0 && latency_status == 0 && first_run_status == 0 && silent_black_status == 0 )); then
    echo "phase2_field_evidence=pass"
    exit 0
fi

echo "phase2_field_evidence=fail"

if (( latency_status == 2 || first_run_status == 2 )); then
    exit 2
fi

exit 1
