//! Computer-use prompt text — compact stable-zone copy + full skill reference.

/// Injected into the **stable** system prompt when `computer_use` is active (~400 tokens).
pub const COMPUTER_USE_GUIDANCE_COMPACT: &str = "\
# Computer Use (macOS, background)\n\
`computer_use` drives the desktop without stealing cursor/focus. Workflow: \
`list_apps` → `launch_app` (browsers: `urls=['about:blank']`) → `focus_app` → \
`capture(mode='som', app='…')` → act by `element` index → verify with another capture.\n\
Rules: always pass `app=` on capture/key/type; never Spotlight-open apps; element indices are \
per-window (re-capture after app change); Terminal: `type` pid-wide (never `element=`). Browsers: \
`navigate` or `launch_app(bundle_id, urls=['https://…'])` — never cmd+l+type+Return (Return does not \
commit omnibox in background). Run `/computer status` or skill **macos-computer-use**.\n";

/// Full operator + agent reference (skill file + `/computer help` parity).
pub const COMPUTER_USE_GUIDANCE_FULL: &str =
    include_str!("../../../../../skills/apple/macos-computer-use/SKILL.md");
