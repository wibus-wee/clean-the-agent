# Clean the Agent GUI

Native macOS frontend for Clean the Agent, built with QuickGUI and Rust. It uses the
root package as a library, so the CLI and desktop app share detection, planning,
concurrency guards, cleanup operations, and preference tweaks.

| Area | Entry point | Responsibility |
| --- | --- | --- |
| Process entry | [`src/main.rs`](./src/main.rs) | Configure application identity, window behavior, and lifecycle entry points. |
| Application | [`src/app`](./src/app) | Own state, background work, page views, native panels, presentation helpers, and UI tests. |
| Design language | [`DESIGN.md`](./DESIGN.md), [`src/design`](./src/design) | Define Geist-inspired tokens, reusable components, and interaction contracts. |
| Icon system | [`src/icons.rs`](./src/icons.rs), [`resources/icons`](./resources/icons) | Resolve native SF Symbols, retain local SVGs, and animate meaningful glyph layers. |
| Cleanup engine | [`../src`](../src) | Detect artifacts, build safety-filtered plans, and apply guarded operations. |
| Packaging | [`quickgui.config.ts`](./quickgui.config.ts) | Define the app name, identifier, Rust entry, and QuickGUI build target. |

## Run

```console
bun install
bun run dev
```

The app scans the current home at launch without changing it. Choose Folder…
uses the native macOS open panel to target another directly accessible home.
Four persistent category tiles explain the 15 Orca artifact types Clean the
Agent can detect and let the user exclude app data, workspaces, integrations,
or agent runtime state from the cleanup plan. Scan results show ownership
evidence and reclaimable bytes. Items that require review remain excluded
unless the user selects the review checkbox.

The interface is provider-first: Orca appears as a first-class navigation item
and owns its scan status and capability scopes. Its upstream application icon
is compiled into the executable from `resources/providers/orca.png`; asset
source, license, and checksum are recorded beside it.

Overview is the default route. Scan All refreshes Orca artifacts and every
registered provider preference recipe in one read-only pass, then presents each
provider as a compact drill-in row. Applying cleanup and changing preferences
remain separate, explicitly confirmed actions.

Codex preferences use the same provider hierarchy. Each setting shows its
effective state, exact configuration target, impact boundary, restart
requirement, and a reversible switch. A settings list and detail pane scale to
additional recipes without stacking large cards. Irreversible cleanup uses a
native confirmation sheet; reversible tweak switches apply directly. Execution
runs outside the UI thread and preserves the engine's scan-time guards; changed
targets fail closed and appear in the result summary. While work runs, an in-window toast shows its
progress and then resolves in place. The toast surface animates in and out, its
loading glyph rotates independently, and success feedback dismisses
automatically; skipped or failed operations also retain their detailed result
panel.

## Verify

```console
bun run check
bun run test
bun run fmt
bun run build
```

QuickGUI currently requires macOS 14 or later, Rust 1.90 or later, Bun 1.4 or
later, and the Xcode Command Line Tools.
