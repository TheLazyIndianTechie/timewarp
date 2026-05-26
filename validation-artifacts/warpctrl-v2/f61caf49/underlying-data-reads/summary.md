# Warp Control CLI f61caf49 underlying-data-read validation
Validated SHA: `f61caf49400dc5c0d37d57a553d27733700e5204`
Artifact owner: `underlying-data-reads`
Artifact root: `validation-artifacts/warpctrl-v2/f61caf49/underlying-data-reads/`
## Result counts
Required cases: 16 pass, 0 fail, 0 skip, 0 blocked.
Visual-inspection failures/blockers: 0.
## What passed
- Built `warpctrl` with `standalone,warp_control_cli` and `warp-oss` with `gui,warp_control_cli`.
- Verified enabled underlying-data reads for `block inspect`, `block output`, `input get`, and `history list` using visible sentinel terminal/input/history state.
- Verified disabled underlying-data-read denial for `block inspect`, `block output`, `input get`, and `history list` with `insufficient_permissions`.
- Verified metadata-read commands `block list`, `file list`, `project active`, and `project list` still work when metadata reads are enabled and underlying-data reads are disabled.
## Caveats
- `file list` returned `files: []` in both enabled and disabled profiles because no editor/CodeView tab was open. The checked CLI exposes `warpctrl file list` only; exploratory `warpctrl file open --help` returned an unrecognized subcommand, so a non-empty file context could not be created via warpctrl setup.
- Active-window read commands require the Warp app window to be active. The first outside-terminal attempt showed `missing_target`; the passing runs use a delayed xterm invocation plus explicit Warp refocus/click before the command executes, while retaining staggered command/UI screenshots.
## Blockers
None.
## Skipped commands
None of the required commands were skipped.
