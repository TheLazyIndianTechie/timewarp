# Warp Control CLI validation summary: discovery-auth-denials
Exact SHA validated: `53d0313df1f712cf98b1c53e9272c588141da350`
Artifact branch: `zach/warpctrl-validation-artifacts/53d0313d/discovery-auth-denials`
Build command: `CARGO_BUILD_JOBS=2 cargo build -p warp --bin warp --bin warpctrl --features gui,warp_control_cli`
Runtime: local-channel Warp app under Xvfb/Openbox, isolated HOME/XDG runtime/discovery directories. `WARPCTRL=/workspace/warp/target/debug/warpctrl`.
## Results
Pass: 15
Fail: 1
Skip: 2
## Findings
- Default-off discovery published a disabled instance record and omitted endpoint/credential broker authority.
- Default-off `instance inspect`, `app ping`, `app version`, `app active`, and `tab create` returned structured `local_control_disabled` denials.
- After enabling outside-Warp control plus metadata-read and app-state-mutation permissions, `instance list`, `instance inspect`, `app ping`, `app version`, and `app active` succeeded.
- `tab create` failed once with `missing_target` when the external terminal had focus, then succeeded when Warp was focused immediately before invocation, creating a new tab (`previous_count=3`, `count=4`).
- `input run "printf warpctrl-validation"` and `drive list --type workflow` returned structured `execution_context_not_allowed` denials without authenticated/inside-Warp authority.
- Running `$WARPCTRL --output-format json app ping` from inside the built Warp terminal succeeded with current implemented behavior because no verified terminal proof env was injected; the CLI used the available outside-Warp metadata grant.
## Skips
- `input run` success path skipped: unsafe high-risk execution path requires authenticated underlying-data authority; denial path captured.
- authenticated `drive list --type workflow` success path skipped: no authenticated-user test identity was available; denial path captured.
## Blockers / caveats
- Active-window selector depends on OS focus. Under Xvfb/Openbox, an external xterm steals focus for terminal screenshots, so the first enabled `tab create` attempt returned `missing_target`. A second run focused Warp immediately before invocation and passed.
## Artifact contents
- Manifest: `validation-artifacts/warpctrl-v2/53d0313d/discovery-auth-denials/manifest.json`
- Screenshots: `validation-artifacts/warpctrl-v2/53d0313d/discovery-auth-denials/screenshots`
- Logs: `validation-artifacts/warpctrl-v2/53d0313d/discovery-auth-denials/logs`
