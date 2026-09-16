# AI Usage Widget — Implementation Plan

> Windows 11 tray widget for monitoring Codex and Claude Code subscription usage.
>
> **Target stack:** Tauri 2 + Vue 3 + TypeScript + Pinia + Tailwind CSS
>
> **Initial platform:** Windows 11
>
> **Plan date:** 2026-09-15

## 1. Product Goal

Build a small Windows desktop utility that:

- starts in the background and lives in the Windows system tray;
- does not show a normal taskbar window;
- opens a compact floating popup above the tray when the tray icon is clicked;
- closes/hides the popup when it loses focus or when `Esc` is pressed;
- detects local Codex CLI and Claude Code installations;
- shows the current 5-hour and weekly usage allowance for each provider;
- shows the reset time/countdown for each available quota window;
- works without asking the user to enter OpenAI or Anthropic credentials into the widget;
- keeps all collected state local to the machine.

The MVP should feel like a native Windows utility rather than a full desktop application.

---

## 2. Important Quota Constraint

The UI should **not claim to know an exact number of “tokens remaining”** for subscription quotas.

Codex and Claude Code subscription limits are not exposed as a simple fixed token bucket. The supported/local surfaces currently expose quota utilization primarily as:

- percentage used;
- percentage remaining, derived as `100 - used_percentage`;
- reset timestamp;
- quota-window duration.

Actual quota consumption can also depend on the model and workload, so converting the remaining percentage into an exact token count would be misleading.

### MVP display

For each provider show values such as:

```text
Codex
5h       72% left     resets in 2h 14m
Weekly   41% left     resets Thu 09:30

Claude
5h       54% left     resets in 1h 03m
Weekly   68% left     resets Sun 18:00
```

If desired later, a separate **estimated token-equivalent** feature can be investigated, but it must be explicitly labelled as an estimate and should not be part of the initial implementation.

---

## 3. Technology Stack

### Desktop shell

- **Tauri 2**
- Rust backend for OS/process/filesystem integration
- Windows WebView2 for the frontend

### Frontend

- **Vue 3**
- **TypeScript**
- Composition API
- **Pinia** for application state
- **Tailwind CSS** for styling
- Vite as the frontend build tool

### Recommended Tauri plugins

- `tauri-plugin-autostart` — launch with Windows
- `tauri-plugin-single-instance` — prevent duplicate tray instances
- `tauri-plugin-store` — small persistent user settings if needed
- `tauri-plugin-shell` only if useful; provider processes can also be managed directly in Rust

Avoid adding a large frontend framework or router unless a later settings screen requires it.

---

## 4. High-Level Architecture

```text
Windows 11
   |
   +-- System Tray
   |      |
   |      +-- click
   |             |
   |             v
   |      Tauri popup window
   |             |
   |             v
   |      Vue + Pinia UI
   |             |
   |             v
   +------ Tauri invoke/events -----------------------+
                                                        |
                                                Rust backend
                                                        |
                           +----------------------------+--------------------------+
                           |                                                       |
                           v                                                       v
                   Codex Provider                                           Claude Provider
                           |                                                       |
                           v                                                       v
                `codex app-server`                                Claude status-line bridge
                JSON-RPC over stdio                              + local cache snapshot
```

The frontend must only consume a normalized provider model. Provider-specific details stay in Rust.

---

## 5. Unified Domain Model

Create one provider-independent quota model.

```ts
export type ProviderId = 'codex' | 'claude'

export type ProviderStatus =
  | 'ready'
  | 'not-installed'
  | 'not-authenticated'
  | 'waiting-for-data'
  | 'stale'
  | 'error'

export interface QuotaWindow {
  usedPercentage: number
  remainingPercentage: number
  resetsAt: number | null       // Unix epoch seconds
  durationMinutes: number | null
}

export interface ProviderQuota {
  provider: ProviderId
  status: ProviderStatus
  fiveHour: QuotaWindow | null
  weekly: QuotaWindow | null
  plan?: string | null
  updatedAt: number | null
  source: string
  error?: string | null
}
```

Never make the Vue layer understand `primary`, `secondary`, Claude status-line payloads, JSON-RPC messages, or provider credentials.

---

## 6. Codex Integration

### Preferred source

Use Codex's local **App Server** rather than parsing terminal output or reading authentication files directly.

The Codex App Server exposes a structured JSON-RPC method:

```text
account/rateLimits/read
```

Its rate-limit snapshots expose fields such as:

- `usedPercent`
- `windowDurationMins`
- `resetsAt`
- `planType`

It can also emit:

```text
account/rateLimits/updated
```

for rolling updates.

### Startup flow

1. Locate the Codex executable.
   - First try normal `PATH` resolution.
   - On Windows, optionally use `where.exe codex` for diagnostics.
2. If missing, set Codex status to `not-installed`.
3. Spawn:

```text
codex app-server
```

using stdio transport.
4. Send the JSON-RPC `initialize` request.
5. Wait for the initialize response.
6. Send the `initialized` notification.
7. Call:

```json
{
  "id": 2,
  "method": "account/rateLimits/read"
}
```

8. Normalize the returned quota windows.
9. Continue listening for `account/rateLimits/updated` notifications.

### Window mapping

Do **not** permanently assume that `primary === 5h` and `secondary === weekly`.

Use `windowDurationMins` whenever available:

```text
300     -> 5-hour window
10080   -> 7-day / weekly window
```

If OpenAI returns only one window, display only that window instead of manufacturing the missing one.

### Refresh strategy

- Keep one App Server child process alive while the widget is running.
- Use push notifications when available.
- On popup open, call `account/rateLimits/read` again if the snapshot is old.
- Add a fallback refresh every ~60 seconds.
- Restart the App Server with bounded backoff if it exits unexpectedly.

### Codex failure states

Handle explicitly:

- Codex is not installed.
- Codex is installed but not signed in with a ChatGPT account.
- App Server protocol/version is incompatible.
- Only the weekly window is available.
- Only the 5-hour window is available.
- Both windows are absent.
- Codex temporarily cannot reach the service.

The UI should degrade to `Unavailable`/`Waiting for data` instead of failing the whole widget.

---

## 7. Claude Code Integration

### Supported data source

Claude Code exposes subscription rate-limit state to its **status-line integration**.

The JSON sent to a configured status-line command can contain:

```json
{
  "rate_limits": {
    "five_hour": {
      "used_percentage": 23.5,
      "resets_at": 1738425600
    },
    "seven_day": {
      "used_percentage": 41.2,
      "resets_at": 1738857600
    }
  }
}
```

For Claude.ai Pro/Max subscriptions this data is available after Claude Code has received at least one API response in the active session.

### MVP strategy: local status-line bridge

Do not scrape Claude's terminal UI and do not extract OAuth credentials.

Instead, make the installed widget executable support a special non-GUI bridge mode:

```text
ai-usage-widget.exe --claude-statusline-bridge
```

When launched in bridge mode, it must:

1. read one Claude status-line JSON payload from `stdin`;
2. extract only the minimal `rate_limits` data required by the widget;
3. write a normalized snapshot atomically to the app's local data directory;
4. optionally forward the same input to the user's previous status-line command;
5. mirror the previous command's stdout so an existing Claude status line keeps working;
6. exit immediately without starting Tauri/WebView.

Example local cache:

```json
{
  "fiveHour": {
    "usedPercentage": 23.5,
    "resetsAt": 1738425600
  },
  "weekly": {
    "usedPercentage": 41.2,
    "resetsAt": 1738857600
  },
  "updatedAt": 1738412345
}
```

### Claude setup flow

During onboarding:

1. Detect `claude` on `PATH`.
2. Locate the user-level Claude settings file under the user's `.claude` directory.
3. Read the current `statusLine` configuration.
4. Ask the user to enable Claude usage integration before modifying that configuration.
5. Store the previous status-line command in the widget's settings.
6. Install the widget bridge command as Claude's status-line command.
7. If a previous command existed, configure bridge forwarding so it is preserved.
8. On integration disable/uninstall, restore the previous status-line configuration when possible.

### Claude refresh behavior

The main Tauri process should watch/read the bridge cache rather than polling Claude itself.

When new data arrives:

- parse the file;
- update the backend provider state;
- emit a Tauri event to Vue;
- update Pinia immediately.

If there is no snapshot yet, display:

```text
Waiting for Claude usage data
Use Claude Code once after enabling integration.
```

If the cache is older than a defined threshold, mark it `stale` but keep showing the last known values.

### Do not use in the MVP

Avoid:

- private/undocumented Anthropic web endpoints;
- reading OAuth tokens from Windows Credential Manager;
- browser-cookie extraction;
- DOM scraping of claude.ai;
- spawning Claude prompts solely to refresh quota state;
- deriving subscription quota from local token transcripts.

Those approaches are either brittle, credential-sensitive, or do not represent the authoritative account-wide quota.

---

## 8. Rust Backend Modules

Suggested structure:

```text
src-tauri/
  src/
    main.rs
    app.rs
    tray.rs
    window.rs
    state.rs
    commands.rs

    providers/
      mod.rs
      types.rs
      codex.rs
      claude.rs

    codex/
      process.rs
      rpc.rs
      protocol.rs

    claude/
      bridge.rs
      config.rs
      cache.rs

    platform/
      windows.rs
```

### Main responsibilities

#### `tray.rs`

- create tray icon;
- handle left click;
- expose right-click menu;
- update tooltip if useful.

#### `window.rs`

- create hidden popup window;
- calculate popup position;
- show/hide/focus;
- hide on focus loss.

#### `providers/codex.rs`

- discover Codex;
- own App Server lifecycle;
- fetch and normalize quotas.

#### `providers/claude.rs`

- discover Claude Code;
- read/watch bridge cache;
- manage integration state.

#### `claude/bridge.rs`

- executable's early bridge-mode entry point;
- parse stdin payload;
- persist minimal quota snapshot;
- forward to existing status-line command when configured.

#### `state.rs`

Shared in-memory normalized provider state protected with appropriate Rust synchronization primitives.

---

## 9. Tauri Commands and Events

Keep the IPC surface small.

### Commands

```text
get_usage_snapshot()
refresh_provider(provider)
refresh_all()
get_settings()
set_start_with_windows(enabled)
enable_claude_integration()
disable_claude_integration()
get_diagnostics()
```

### Events

```text
usage://updated
provider://status-changed
```

Vue should subscribe once at application startup and push updates into Pinia.

---

## 10. Pinia Stores

### `useUsageStore`

State:

```ts
{
  providers: {
    codex: ProviderQuota,
    claude: ProviderQuota
  },
  refreshing: boolean
}
```

Actions:

```text
initialize()
refreshAll()
refreshProvider(id)
applyProviderUpdate(payload)
```

Getters:

```text
codex
claude
hasAnyAvailableProvider
```

### `useSettingsStore`

Suggested settings:

```text
startWithWindows
hideOnBlur
refreshIntervalSeconds
claudeIntegrationEnabled
```

Avoid persisting quota snapshots in Pinia storage. Quota state is transient/provider-owned.

---

## 11. Popup Window Behavior

The window should behave like a small Windows widget.

### Initial configuration

Approximate initial dimensions:

```text
width:  380 px
height: 330-420 px depending on content
```

Tauri window characteristics:

- start hidden;
- no decorations/title bar;
- not resizable;
- not maximizable;
- not minimizable;
- skip taskbar;
- always on top while visible;
- no normal application navigation chrome.

Avoid transparent WebView backgrounds unless they are actually needed; a normal opaque rounded panel is simpler and more reliable on Windows.

### Tray click

On left click:

```text
if popup visible:
    hide popup
else:
    calculate anchor position
    move popup above tray click area
    show popup
    focus popup
    refresh stale quota data
```

### Positioning

Use the tray event's screen coordinates when available.

Requirements:

- support 100%, 125%, 150%, 200% Windows scaling;
- handle secondary monitors;
- keep the entire popup inside the active monitor's work area;
- prefer positioning directly above the tray icon;
- if insufficient vertical space exists, place the popup on the nearest valid side.

Do not hard-code `screenWidth - popupWidth` as the primary positioning strategy.

### Dismissal

Hide the window when:

- it loses focus;
- the user presses `Esc`;
- the tray icon is clicked again.

Do not destroy/recreate the WebView on each click.

---

## 12. Tray Menu

Right-click menu:

```text
Open
Refresh
----------------
Start with Windows     [checkable]
Claude integration...  [state/action]
----------------
Quit
```

`Quit` must terminate child processes and file watchers cleanly.

Closing/hiding the popup must **not** quit the application.

---

## 13. UI Layout

Single compact view.

```text
+--------------------------------------+
| AI Usage                      refresh|
|                                      |
| CODEX                                |
| 5h       [=========---]   72% left   |
|          resets in 2h 14m            |
| Weekly   [======------]   41% left   |
|          Thu 09:30                   |
|                                      |
| CLAUDE                               |
| 5h       [=======-----]   54% left   |
|          resets in 1h 03m            |
| Weekly   [========----]   68% left   |
|          Sun 18:00                   |
|                                      |
| Updated 10s ago                      |
+--------------------------------------+
```

### Components

```text
App.vue
  WidgetHeader.vue
  ProviderCard.vue
    QuotaRow.vue
    ProgressBar.vue
  ProviderStatus.vue
  WidgetFooter.vue
```

### Tailwind usage

Use Tailwind for:

- compact spacing;
- rounded panel/card treatment;
- typography;
- progress bars;
- hover/focus states;
- dark/light palettes if both are later supported.

For MVP, a polished dark theme is enough if theme switching is not a product requirement.

---

## 14. Quota Presentation Rules

### Percentage

```ts
remaining = clamp(100 - usedPercentage, 0, 100)
```

Display the provider's actual precision internally but round UI values sensibly.

### Reset countdown

The backend stores absolute reset timestamps.

The frontend can update human-readable countdowns every second without re-fetching provider data:

```text
2h 14m
48m
12m
```

When the reset timestamp passes:

- mark the window as needing refresh;
- trigger a provider refresh;
- do not automatically set usage to `0%` until the provider confirms it.

### Missing window

If one quota window is absent:

```text
5h       Not reported
Weekly   37% left
```

Do not treat missing data as 100% remaining.

---

## 15. Refresh and Staleness Policy

### Codex

- event-driven updates from App Server where possible;
- full snapshot on startup;
- full snapshot when popup opens if stale;
- fallback poll approximately every 60 seconds.

### Claude

- event-driven through the status-line bridge cache;
- filesystem watcher for fast updates;
- reload snapshot when popup opens;
- mark stale if no bridge update has occurred for a reasonable interval while Claude is expected to be active.

### UI

Show small status metadata rather than silently using old values:

```text
Updated just now
Updated 3m ago
Stale · last update 42m ago
```

---

## 16. Startup and Background Operation

### App startup

1. Ensure a single instance.
2. Do not show the popup automatically.
3. Create system tray.
4. Initialize shared state.
5. Start Codex provider asynchronously.
6. Start Claude cache watcher asynchronously.
7. Load settings.
8. Wait for tray interaction.

The frontend WebView may be created at startup and kept hidden for instant popup response.

### Start with Windows

Use Tauri's autostart plugin.

Default can be either disabled or enabled during first-run onboarding, but the setting must be visible and reversible.

---

## 17. Privacy and Security

The widget should be designed as a local-only utility.

### Requirements

- Do not upload usage data anywhere.
- Do not send analytics in the MVP.
- Do not read Codex `auth.json` directly when App Server can provide the required state.
- Do not extract Anthropic OAuth credentials.
- Do not persist Claude conversation text.
- Do not persist Codex conversation text.
- Claude bridge cache should contain only quota metadata and timestamps.
- Avoid logging raw provider payloads in production unless diagnostic logging is explicitly enabled.
- Redact paths/account identifiers from user-facing diagnostics where possible.

### Atomic cache writes

For the Claude bridge:

```text
write temp file
fsync/close
rename over current snapshot
```

This prevents the main app from reading a partially written JSON document.

---

## 18. Diagnostics

Add a small diagnostics model even if there is no full settings page initially.

Useful fields:

```text
Codex executable path
Codex CLI version
Codex provider status
Codex App Server status
Claude executable path
Claude Code version
Claude integration enabled
Claude cache last updated
Application version
```

Do not include auth tokens.

This will greatly reduce debugging friction when provider versions change.

---

## 19. Testing Strategy

### Rust unit tests

Test:

- Codex rate-limit response parsing;
- mapping 300-minute and 10,080-minute windows;
- missing/null windows;
- percentage clamping;
- Claude status-line payload parsing;
- malformed bridge input;
- atomic cache serialization;
- existing Claude status-line forwarding configuration.

### Provider integration tests

Use fixtures/mocks instead of consuming real quota.

Codex mock cases:

- 5h + weekly;
- weekly only;
- no quota;
- not authenticated;
- child process exit;
- malformed JSON-RPC message;
- sparse `account/rateLimits/updated` notification.

Claude bridge fixtures:

- both windows;
- 5h only;
- weekly only;
- `rate_limits` absent;
- first-call state;
- malformed stdin.

### Vue tests

Test:

- provider card rendering;
- percentage conversion;
- unavailable/stale states;
- reset countdown formatting;
- Pinia provider updates.

### Windows manual acceptance tests

Test on Windows 11 with:

- 100% display scaling;
- 125% display scaling;
- 150% display scaling;
- multiple monitors;
- taskbar on primary monitor;
- app launched at login;
- Codex not installed;
- Claude not installed;
- provider installed but logged out;
- popup focus loss;
- tray icon click toggling;
- application update/restart.

---

## 20. Packaging

Target Windows first.

Use normal Tauri Windows packaging and produce a signed installer when the project is ready for distribution.

Build requirements should include:

- x64 initially;
- optional ARM64 later;
- WebView2 availability handled according to Tauri's standard Windows deployment strategy;
- tray `.ico` with multiple embedded sizes.

Do not bundle Codex or Claude Code themselves. The widget detects the user's existing installations.

---

## 21. Implementation Milestones

### Milestone 1 — Desktop shell

- Scaffold Tauri 2 + Vue 3 + TypeScript.
- Add Pinia.
- Add Tailwind CSS.
- Create tray icon.
- Create hidden frameless popup.
- Implement tray-click positioning.
- Hide on blur / `Esc`.
- Add single-instance behavior.

**Done when:** a dummy popup reliably behaves like a Windows tray widget.

### Milestone 2 — Provider domain layer

- Add normalized Rust quota types.
- Add frontend TypeScript equivalents.
- Create Tauri snapshot command/events.
- Create Pinia usage store.
- Render mocked Codex/Claude provider cards.

**Done when:** the UI is completely provider-agnostic.

### Milestone 3 — Codex provider

- Detect Codex installation.
- Spawn App Server.
- Implement JSON-RPC transport.
- Implement initialize handshake.
- Call `account/rateLimits/read`.
- Map 5h/weekly windows by duration.
- Listen for rate-limit updates.
- Add reconnect/backoff and errors.

**Done when:** real Codex quota percentages and reset times appear in the widget.

### Milestone 4 — Claude provider

- Detect Claude Code installation.
- Implement bridge-mode executable path.
- Parse Claude status-line stdin JSON.
- Write minimal local quota snapshot.
- Watch snapshot from main Tauri process.
- Implement opt-in status-line configuration.
- Preserve/forward an existing user status line.
- Implement disable/restore path.

**Done when:** real Claude 5h/weekly percentages and reset times appear after Claude Code activity.

### Milestone 5 — UX and resilience

- Add loading/unavailable/stale states.
- Add reset countdowns.
- Add manual refresh.
- Add diagnostics.
- Add start-with-Windows option.
- Test multi-monitor/DPI positioning.

### Milestone 6 — Release

- Production logging policy.
- Installer.
- Icons/resources.
- Windows code signing.
- Smoke test on clean Windows account.
- Version/provider compatibility notes.

---

## 22. MVP Acceptance Criteria

The MVP is complete when all of the following are true:

- [ ] App starts without opening a normal window.
- [ ] App remains available from the Windows 11 system tray.
- [ ] Left-clicking the tray icon opens the popup above the tray.
- [ ] Clicking the tray icon again hides it.
- [ ] Popup hides when focus is lost.
- [ ] Popup does not appear as a normal taskbar application window.
- [ ] Codex installation is detected automatically.
- [ ] Codex 5-hour quota is shown when reported.
- [ ] Codex weekly quota is shown when reported.
- [ ] Codex reset timestamps/countdowns are shown.
- [ ] Claude Code installation is detected automatically.
- [ ] Claude integration can be enabled explicitly.
- [ ] Claude 5-hour quota is shown when reported.
- [ ] Claude weekly quota is shown when reported.
- [ ] Claude reset timestamps/countdowns are shown.
- [ ] Existing Claude status-line behavior is preserved when integration is enabled.
- [ ] Missing provider/window data is clearly represented as unavailable rather than zero/full.
- [ ] No OpenAI/Anthropic credential is copied into application settings.
- [ ] No conversation content is persisted by the widget.
- [ ] App can start automatically with Windows.
- [ ] App works correctly at common Windows DPI scales.

---

## 23. Explicit Non-Goals for MVP

Do not build these initially:

- macOS/Linux support;
- exact subscription “tokens remaining” estimates;
- API billing/cost dashboards;
- per-project token analytics;
- historical charts;
- cloud sync;
- user accounts for this widget;
- provider credential management;
- browser scraping;
- automatic quota purchases/resets;
- Windows Widgets Board integration.

The first version should remain a focused tray utility.

---

## 24. Suggested First Implementation Order

Start in this exact order:

1. Tauri tray + floating popup behavior.
2. Vue/Pinia/Tailwind UI with mock data.
3. Shared normalized quota model.
4. Codex App Server client.
5. Real Codex card.
6. Claude bridge executable mode.
7. Claude settings integration and cache watcher.
8. Real Claude card.
9. Autostart, diagnostics, stale states.
10. Packaging and Windows acceptance testing.

This sequence isolates the riskiest provider integrations from the UI and prevents provider-specific logic from leaking into Vue.

---

## 25. Technical References

Current implementation assumptions are based on the providers' current local interfaces and documentation and should be revalidated when implementation begins.

- OpenAI Codex App Server — JSON-RPC initialization and `account/rateLimits/read`:
  https://github.com/openai/codex/tree/main/codex-rs/app-server

- OpenAI Codex App Server protocol source:
  https://github.com/openai/codex

- OpenAI Codex usage limits:
  https://help.openai.com/en/articles/11369540

- Claude Code status-line documentation and `rate_limits` fields:
  https://code.claude.com/docs/en/statusline

- Claude Code usage/cost documentation:
  https://code.claude.com/docs/en/costs

