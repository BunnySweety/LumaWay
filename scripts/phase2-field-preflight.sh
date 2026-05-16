#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

min_fps="120"
require_camera="yes"
video_device=""
camera_fps=""

usage() {
    cat <<'EOF'
Usage:
  scripts/phase2-field-preflight.sh [--min-fps <fps>] [--camera-fps <fps>] [--require-camera yes|no] [--video-device <path>]

Options:
  --min-fps <fps>          Minimum camera frame rate required for Phase 2 video evidence. Default: 120.
  --camera-fps <fps>       Declared capture frame rate for a phone or non-V4L2 camera.
                           Pair with --require-camera no when no /dev/video* device is expected.
  --require-camera yes|no  Whether a /dev/video* camera must be present. Default: yes.
  --video-device <path>    Check one explicit camera path instead of scanning /dev/video*.

This command does not prove Phase 2. It checks whether the local environment has
the helper scripts, harness files, and camera preconditions needed to collect
the external field evidence.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --min-fps)
            [[ $# -ge 2 ]] || {
                echo "missing value for --min-fps" >&2
                exit 2
            }
            min_fps="$2"
            shift 2
            ;;
        --require-camera)
            [[ $# -ge 2 ]] || {
                echo "missing value for --require-camera" >&2
                exit 2
            }
            require_camera="$2"
            shift 2
            ;;
        --camera-fps)
            [[ $# -ge 2 ]] || {
                echo "missing value for --camera-fps" >&2
                exit 2
            }
            camera_fps="$2"
            shift 2
            ;;
        --video-device)
            [[ $# -ge 2 ]] || {
                echo "missing value for --video-device" >&2
                exit 2
            }
            video_device="$2"
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

if [[ ! "$min_fps" =~ $number_re ]]; then
    echo "--min-fps must be a positive number" >&2
    exit 2
fi

awk -v value="$min_fps" 'BEGIN { exit !(value > 0) }' || {
    echo "--min-fps must be greater than zero" >&2
    exit 2
}

if [[ -n "$camera_fps" && ! "$camera_fps" =~ $number_re ]]; then
    echo "--camera-fps must be a positive number" >&2
    exit 2
fi

if [[ -n "$camera_fps" ]]; then
    awk -v value="$camera_fps" 'BEGIN { exit !(value > 0) }' || {
        echo "--camera-fps must be greater than zero" >&2
        exit 2
    }
fi

case "$require_camera" in
    yes|no)
        ;;
    *)
        echo "--require-camera must be yes or no" >&2
        exit 2
        ;;
esac

status=0

echo "phase2_field_preflight"
printf "min_fps=%s require_camera=%s\n" "$min_fps" "$require_camera"
if [[ -n "$camera_fps" ]]; then
    printf "declared_camera_fps=%s\n" "$camera_fps"
fi

check_file() {
    local label="$1"
    local path="$2"

    if [[ -f "$path" ]]; then
        printf "%s=pass path=%s\n" "$label" "$path"
    else
        printf "%s=fail path=%s\n" "$label" "$path"
        status=1
    fi
}

check_executable() {
    local label="$1"
    local path="$2"

    if [[ -x "$path" ]]; then
        printf "%s=pass path=%s\n" "$label" "$path"
    else
        printf "%s=fail path=%s\n" "$label" "$path"
        status=1
    fi
}

check_executable "helper_latency" "$script_dir/phase2-latency-summary.sh"
check_executable "helper_first_run" "$script_dir/phase2-first-run-summary.sh"
check_executable "helper_field_evidence" "$script_dir/phase2-field-evidence.sh"
check_file "harness_doc" "$repo_root/docs/phase2-comparison-harness.md"
check_file "harness_fixture" "$repo_root/docs/fixtures/phase2-patterns.html"

usable_devices=0

if [[ "$require_camera" == "no" ]]; then
    echo "camera=skip reason=require_camera_no"
else
    declare -a devices=()

    if [[ -n "$video_device" ]]; then
        devices=("$video_device")
    else
        while IFS= read -r device; do
            devices+=("$device")
        done < <(find /dev -maxdepth 1 -name 'video*' -print | sort)
    fi

    if (( ${#devices[@]} == 0 )); then
        echo "camera=fail reason=no_video_device"
        status=1
    else
        usable_devices=0
        for device in "${devices[@]}"; do
            if [[ -e "$device" && "$device" == /dev/video* ]]; then
                printf "camera_device=pass path=%s\n" "$device"
                usable_devices=$((usable_devices + 1))
            else
                printf "camera_device=fail path=%s\n" "$device"
                status=1
            fi
        done

        if (( usable_devices == 0 )); then
            echo "camera=fail reason=no_usable_video_device"
            status=1
        else
            printf "camera=pass devices=%s\n" "$usable_devices"
        fi
    fi
fi

if [[ -n "$camera_fps" ]]; then
    if awk -v fps="$camera_fps" -v min="$min_fps" 'BEGIN { exit !(fps >= min) }'; then
        printf "camera_fps=pass fps=%s min_fps=%s source=declared\n" "$camera_fps" "$min_fps"
    else
        printf "camera_fps=fail fps=%s min_fps=%s source=declared\n" "$camera_fps" "$min_fps"
        status=1
    fi
elif (( usable_devices > 0 )); then
    if command -v v4l2-ctl >/dev/null 2>&1; then
        fps_pass=0
        for device in "${devices[@]}"; do
            [[ -e "$device" && "$device" == /dev/video* ]] || continue

            v4l2_output="$(v4l2-ctl --device "$device" --list-formats-ext 2>/dev/null || true)"
            max_device_fps="$(printf "%s\n" "$v4l2_output" \
                | sed -nE 's/.*\(([0-9]+([.][0-9]+)?) fps\).*/\1/p' \
                | awk 'NF { if ($1 > max) max = $1 } END { if (max != "") printf "%.3f", max }')"

            if [[ -z "$max_device_fps" ]]; then
                printf "camera_fps=warn path=%s reason=no_v4l2_fps_reported min_fps=%s\n" "$device" "$min_fps"
            elif awk -v fps="$max_device_fps" -v min="$min_fps" 'BEGIN { exit !(fps >= min) }'; then
                printf "camera_fps=pass path=%s max_fps=%s min_fps=%s source=v4l2\n" "$device" "$max_device_fps" "$min_fps"
                fps_pass=1
            else
                printf "camera_fps=fail path=%s max_fps=%s min_fps=%s source=v4l2\n" "$device" "$max_device_fps" "$min_fps"
            fi
        done

        if (( fps_pass == 0 )); then
            status=1
        fi
    else
        echo "camera_fps=fail reason=not_checked_missing_v4l2_ctl hint=install_v4l_utils_or_pass_camera_fps"
        status=1
    fi
else
    echo "camera_fps=skip reason=no_camera_to_check"
fi

if command -v ffprobe >/dev/null 2>&1; then
    printf "tool_ffprobe=pass path=%s\n" "$(command -v ffprobe)"
else
    echo "tool_ffprobe=warn reason=missing_optional_video_inspection_tool"
fi

if command -v ffmpeg >/dev/null 2>&1; then
    printf "tool_ffmpeg=pass path=%s\n" "$(command -v ffmpeg)"
else
    echo "tool_ffmpeg=warn reason=missing_optional_video_conversion_tool"
fi

echo "manual_evidence_required=visible_latency,no_silent_black,first_run_timing"

if (( status == 0 )); then
    echo "phase2_field_preflight=pass"
    exit 0
fi

echo "phase2_field_preflight=fail"
exit 1
