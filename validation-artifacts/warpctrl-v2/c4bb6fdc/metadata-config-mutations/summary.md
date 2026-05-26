# Warp Control CLI validation: metadata/config mutations
Validated SHA: `c4bb6fdc670d667e78041a9318eda7c6778a22a8`
Expected SHA: `c4bb6fdc670d667e78041a9318eda7c6778a22a8`
SHA match: `True`
Artifact root: `validation-artifacts/warpctrl-v2/c4bb6fdc/metadata-config-mutations`
## Counts
Required commands: 20 pass, 0 fail, 0 skip, 20 total.
All attempted commands, including restoration: 21 pass, 0 fail, 0 skip, 21 total.
## Blockers
None.
## Notes
- Built `warp`, `warp-oss`, and standalone `warpctrl` with `warp_control_cli` compiled in.
- Graphical validation used `target/debug/warp-oss` under Xvfb/Openbox because the non-bundled local-channel `warp` binary requires the internal `warp-channel-config` helper at runtime in this sandbox.
- Outside-Warp Scripting permissions were enabled in an isolated Linux private preferences file before app launch: outside-Warp control, metadata reads, and metadata/configuration mutations.
- Every executed `warpctrl` invocation has a terminal screenshot using a combined staggered xterm + Warp app composition.
- `keybinding get copy` preserved the known behavior: JSON `missing_target` error and exit code 1.
- `terminal.input.syntax_highlighting` was toggled for validation and restored to its original value with an extra restoration command.
## Skipped commands
None.
