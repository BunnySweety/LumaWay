---
name: Phase 2 field validation
about: Record visible latency, first-run, and no-silent-black evidence for Phase 2
title: "Phase 2 field validation evidence"
---

## Context

Phase 2 code, docs, validation helpers, and available TV diagnostics are delivered. The remaining completion gate is external field evidence that requires a person and/or camera.

Relevant artifacts:

- `docs/phase2-comparison-harness.md`
- `docs/phase2-validation-2026-05-16.md`
- `scripts/phase2-field-preflight.sh`
- `scripts/phase2-field-evidence.sh`

## Required Evidence

- [ ] Run the field preflight on the target machine before recording.
- [ ] Record the `Latency flash` pattern with the screen and Hue lights visible in the same video at 120 fps or higher.
- [ ] Measure at least 5 accepted full-screen black/white transitions from `screen_frame:light_frame`.
- [ ] Confirm no non-black pattern stayed black silently during the run.
- [ ] Time a new-user installed-app flow from launching LumaWay to first satisfactory non-black TV/monitor sync.
- [ ] Confirm `calibrate-capture` was not required for the timed flow.

For a phone or another non-V4L2 camera, use:

```sh
scripts/phase2-field-preflight.sh --require-camera no --camera-fps 120 | tee phase2-preflight.txt
```

For a local `/dev/video*` camera, use:

```sh
scripts/phase2-field-preflight.sh | tee phase2-preflight.txt
```

- [ ] Run the combined verifier:

```sh
scripts/phase2-field-evidence.sh --fps 120 --elapsed-seconds <seconds> --preflight-output phase2-preflight.txt --calibrate-used no --silent-black no <screen_frame:light_frame>...
```

Use exactly one preflight option. The combined verifier exits non-zero when the
saved preflight output is missing, ambiguous, or did not pass, when both
`--preflight-output` and `--preflight-pass` are provided, or when `--fps` is
below the default 120 fps minimum, even if the frame pairs would otherwise
satisfy the latency threshold.

## Preflight Block

Paste the preflight output here:

```text
phase2_field_preflight
...
```

## Evidence Block

Paste the verifier output here:

```text
phase2_field_evidence
...
```

## Completion Rule

Phase 2 should not be marked complete until the verifier prints `phase2_field_evidence=pass` and the measured evidence is recorded in `docs/phase2-validation-2026-05-16.md`.

If any gate fails, keep Phase 2 open and tune the relevant capture/sampling/defaults before re-running this checklist.
