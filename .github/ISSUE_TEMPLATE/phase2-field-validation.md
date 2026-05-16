---
name: Phase 2 field validation
about: Record visible latency and first-run evidence for Phase 2
title: "Phase 2 field validation evidence"
---

## Context

Phase 2 code, docs, validation helpers, and available TV diagnostics are delivered. The remaining completion gate is external field evidence that requires a person and/or camera.

Relevant artifacts:

- `docs/phase2-comparison-harness.md`
- `docs/phase2-validation-2026-05-16.md`
- `scripts/phase2-field-evidence.sh`

## Required Evidence

- [ ] Record the `Latency flash` pattern with the screen and Hue lights visible in the same video at 120 fps or higher.
- [ ] Measure at least 5 accepted full-screen black/white transitions from `screen_frame:light_frame`.
- [ ] Confirm no non-black pattern stayed black silently during the run.
- [ ] Time a new-user installed-app flow from launching LumaWay to first satisfactory non-black TV/monitor sync.
- [ ] Confirm `calibrate-capture` was not required for the timed flow.
- [ ] Run the combined verifier:

```sh
scripts/phase2-field-evidence.sh --fps 120 --elapsed-seconds <seconds> --calibrate-used no --silent-black no <screen_frame:light_frame>...
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
