#!/usr/bin/env bash
set -euo pipefail

threshold_ms="300"
min_transitions="5"
fps=""
declare -a pairs=()

usage() {
    cat <<'EOF'
Usage:
  scripts/phase2-latency-summary.sh --fps <fps> [--threshold-ms <ms>] [--min-transitions <n>] <screen_frame:light_frame>...

Example:
  scripts/phase2-latency-summary.sh --fps 120 100:124 220:247 340:369 460:490 580:613

Each pair is measured from the first video frame where the screen changes to the
first frame where any target Hue light visibly changes.
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
        --threshold-ms)
            [[ $# -ge 2 ]] || {
                echo "missing value for --threshold-ms" >&2
                exit 2
            }
            threshold_ms="$2"
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

number_re='^[0-9]+([.][0-9]+)?$'
integer_re='^[0-9]+$'

if [[ -z "$fps" || ! "$fps" =~ $number_re ]]; then
    echo "--fps must be a positive number" >&2
    exit 2
fi

if [[ ! "$threshold_ms" =~ $number_re ]]; then
    echo "--threshold-ms must be a positive number" >&2
    exit 2
fi

if [[ ! "$min_transitions" =~ $integer_re ]]; then
    echo "--min-transitions must be a positive integer" >&2
    exit 2
fi

min_transitions_value=$((10#$min_transitions))

if (( min_transitions_value <= 0 )); then
    echo "--min-transitions must be greater than zero" >&2
    exit 2
fi

awk -v value="$fps" 'BEGIN { exit !(value > 0) }' || {
    echo "--fps must be greater than zero" >&2
    exit 2
}

awk -v value="$threshold_ms" 'BEGIN { exit !(value > 0) }' || {
    echo "--threshold-ms must be greater than zero" >&2
    exit 2
}

if (( ${#pairs[@]} < min_transitions_value )); then
    echo "need at least ${min_transitions_value} transitions, got ${#pairs[@]}" >&2
    exit 1
fi

printf "fps=%s threshold_ms=%s transitions=%s\n" "$fps" "$threshold_ms" "${#pairs[@]}"
printf "%-10s %-12s %-12s %-12s %-8s\n" "transition" "screen_frame" "light_frame" "latency_ms" "result"

failures=0
index=1
for pair in "${pairs[@]}"; do
    if [[ "$pair" != *:* ]]; then
        echo "invalid transition '$pair'; expected screen_frame:light_frame" >&2
        exit 2
    fi

    screen_frame="${pair%%:*}"
    light_frame="${pair#*:}"

    if [[ ! "$screen_frame" =~ $integer_re || ! "$light_frame" =~ $integer_re ]]; then
        echo "invalid transition '$pair'; frames must be non-negative integers" >&2
        exit 2
    fi

    if (( light_frame < screen_frame )); then
        echo "invalid transition '$pair'; light frame is before screen frame" >&2
        exit 2
    fi

    delta_frames=$((light_frame - screen_frame))
    latency_ms="$(awk -v frames="$delta_frames" -v fps="$fps" 'BEGIN { printf "%.3f", frames * 1000 / fps }')"
    result="$(awk -v latency="$latency_ms" -v threshold="$threshold_ms" 'BEGIN { print (latency <= threshold) ? "pass" : "fail" }')"

    if [[ "$result" == "fail" ]]; then
        failures=$((failures + 1))
    fi

    printf "%-10s %-12s %-12s %-12s %-8s\n" "$index" "$screen_frame" "$light_frame" "$latency_ms" "$result"
    index=$((index + 1))
done

if (( failures > 0 )); then
    echo "phase2_latency=fail failures=$failures"
    exit 1
fi

echo "phase2_latency=pass"
