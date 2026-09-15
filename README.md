<p align="center">
  <p align="center">
    <img src="./gui/resources/app/icon.png" alt="Preview" width="128" />
  </p>
  <h1 align="center"><b>Clean the Agent</b></h1>
  <p align="center">
    A conservative cleanup tool for artifacts and configuration changes left by developer tools.
    <br />
    <br />
    <b>Download for </b>
    <del>
    <i>
    macOS
    ·
    Windows
    ·
    Linux
    </i>
    </del>
    <br />
  </p>
</p>

## Motivation

Clean the Agent was created to address the problem of leftover artifacts and configuration changes from developer tools. These remnants can clutter the development environment, cause conflicts, and lead to unexpected behavior. 

By providing a conservative cleanup approach, the tool ensures that only safe and necessary actions are taken, minimizing the risk of disrupting the developer's workflow.

## Overview

The workflow is deliberately staged:

```text
detect -> inspect -> classify and explain -> plan -> apply
```

Detection is read-only. `clean` is a dry run unless `--apply` is supplied.
Interactive applies show the plan and use the same confirmation prompt for
cleanup and tweaks. Non-interactive callers must also pass `--yes`.

## Project Map

| Area | Location | Responsibility |
| --- | --- | --- |
| CLI | [`src/main.rs`](./src/main.rs), [`src/cli_interactive.rs`](./src/cli_interactive.rs), [`src/cli_output.rs`](./src/cli_output.rs) | Parse commands, run guided prompts, and send cleanup and tweak results through one human/JSON presentation pipeline. |
| Desktop GUI | [`gui`](./gui) | Provide a native macOS workflow for scanning, reviewing plans, applying cleanup, and managing tweaks. |
| Provider contract | [`src/provider.rs`](./src/provider.rs) | Define the read-only `Provider` trait, platform context, and local/WSL/SSH home scopes. |
| Finding model | [`src/model.rs`](./src/model.rs) | Represent ownership, safety, evidence, scope, snapshots, plans, and typed actions. |
| Engine | [`src/engine.rs`](./src/engine.rs) | Run providers, exclude review-gated actions by default, order dependencies, and report results. |
| Operations | [`src/operations.rs`](./src/operations.rs) | Apply hash-guarded file rewrites, snapshot-guarded removals, and guarded Git worktree removal. |
| Tweaks | [`src/tweak.rs`](./src/tweak.rs), [`src/tweaks`](./src/tweaks) | Inspect explicit desired-state recipes without including them in automatic cleanup. |
| Orca composition | [`src/providers/orca/mod.rs`](./src/providers/orca/mod.rs) | Compose independent Orca detectors without exposing Orca paths to the engine. |
| Orca detectors | [`src/providers/orca`](./src/providers/orca) | Own config, state, worktree, runtime, integration, trust, skill, path, and evidence rules. |

## Usage

Build, scan, and inspect the automatic cleanup plan:

```console
cargo build
cargo run
cargo run -- scan
cargo run -- clean
```

Running without a subcommand opens a searchable terminal menu for scanning,
cleaning safe artifacts, or applying a preference tweak. The same menu is
available explicitly with `clean-the-agent interactive`. Use the arrow keys or type
to filter, press Enter to select, and press Esc to cancel.

The previous `clean-any` executable remains available as a compatibility alias.

Run the native macOS GUI:

```console
cd gui
bun install
bun run dev
```

The GUI scans the current home or a folder selected with the native macOS open
panel. It presents supported artifact categories before scanning, lets users
scope the cleanup plan, shows findings in a list-detail view, keeps
review-required items excluded by default, confirms cleanup in a native sheet,
and reports partial apply failures. The Tweaks view exposes the same explicit
preference recipes as the CLI.

Apply only the automatic plan after reviewing it:

```console
cargo run -- clean --apply
```

Consequential state, including live worktrees, backups, terminal history,
diverged copies, and live trust entries, requires a second opt-in:

```console
cargo run -- clean --include-review
cargo run -- clean --include-review --apply
```

Use `--verbose` to include ownership, scope, and detection evidence. Use
`--json` for machine-readable output; `--json --apply` requires `--yes` because
JSON output cannot prompt. Explicit roots support fixtures, mounted homes, WSL
distributions, and mounted SSH filesystems:

```console
cargo run -- --home /mounted/home scan --json
cargo run -- --orca-data-dir /var/lib/orca scan
cargo run -- --workspace-root /workspaces clean
cargo run -- --wsl-home /mnt/wsl/home/alice scan
cargo run -- --remote-home /mnt/ssh/server/home/alice scan
cargo run -- --stale-days 14 clean
```

Preference tweaks are listed separately and run as dry-runs unless explicitly
applied. For example, the Codex Pet tweak preserves every unrelated binding and
writes the same null-binding form used by Codex:

```console
cargo run -- tweaks
cargo run -- tweak codex.disable-pet-shortcut
cargo run -- tweak codex.disable-pet-shortcut --apply
```

The tweak resolves `$CODEX_HOME/keybindings.json`, or
`~/.codex/keybindings.json` when `CODEX_HOME` is unset. It refuses malformed or
concurrently changed files. Restart Codex after applying it.

`--remote-home` does not open an SSH connection. It labels and scans a remote
home that is directly accessible to the process. The same binary can also run
on the remote host itself.

## Orca Coverage

| Surface | Detection and cleanup |
| --- | --- |
| Workspaces | Discovers the default `~/orca/workspaces`, explicit and persisted roots, flat and nested `.orca-worktree-trash`, and Orca-provenance worktrees. Worktrees are review-gated and must still be clean and Git-registered. Generated trash is automatic. |
| Orca data | Separates `orca-data.json`, `orca-github-cache.json`, Electron caches, temporary data, terminal history, `.pending-delete` tombstones, logs, and unknown owned data. Persistent state is never classified as cache. |
| Log retention | Automatically plans files older than `--stale-days`; recent logs remain informational. |
| Codex runtime | Inspects `codex-runtime-home/home` and `codex-session-backfill`. Links and copies equal to the real Codex resource are automatic; diverged or sole copies require review. |
| Managed hooks | Removes recognized scripts and ownership records under `~/.orca/agent-hooks` after configuration rewrites are ordered first. Unknown files require review. |
| JSON and JSONC mutations | Structurally removes exact Orca commands from Claude Code, OpenClaude, Codex hooks, Cursor, Gemini, Antigravity, Droid/Factory, Command Code, Copilot, Grok, and Devin. JSONC comments are retained. Orca-named Copilot and Grok hook files are removed only when no unrelated content remains. |
| Trust state | Correlates Cursor markers, Copilot `trustedFolders`, and Codex TOML project trust with Orca workspace provenance. Missing workspaces are automatic; live workspace trust requires review. Codex hook trust blocks must reference Orca's hook directory. |
| Other integrations | Removes Kimi's bounded managed TOML block, the marked Amp plugin, and the marked Hermes plugin while disabling only `orca-status` in Hermes YAML. Kimi and Hermes `.bak` files require review. Environment-specific homes are honored for the current user. |
| Skills | Attributes only provider links and copies whose `skills` v3 lock entry names `stablyai/orca` and whose Git tree hash matches Orca's versioned skill registry through 1.4.197. Personal skills, metadata, generic Orca install receipts, modified copies, and unknown revisions are left untouched. A newer observed Orca version produces a coverage warning. |
| WSL and SSH | Scans explicitly supplied home scopes, including mirrored workspace trash, remote agent configuration, remote hook scripts, and versioned `.orca-remote/relay-*` and `orcad-*` state. Reports retain the scope. |
| MCP files | `.mcp.json`, `.cursor/mcp.json`, `.claude.json`, and `.claude/mcp.json` are intentionally outside Orca cleanup because their creation is explicit user state. |

Malformed structured configuration is reported and never planned. Ownership is
not inferred merely because a filename or value contains the word `orca`.

## Safety Model

| Safety | Meaning | Planning behavior |
| --- | --- | --- |
| `automatic` | Ownership and the exact removable unit are proven, or stale state has an exact generated form. | Included by default. |
| `review_required` | Attribution exists, but removal may discard useful history, a live trust decision, a diverged copy, or a worktree. | Included only with `--include-review`. |
| `informational` | The state matters to the explanation but no conservative cleanup action is available. | Never applied. |

Every finding also records one of three ownership levels: provider-owned,
injected by the provider, or attributed through provider provenance.

File rewrites are atomic and guarded by the scan-time SHA-256. Path removals
use a recursive non-following snapshot. A changed target fails closed and must
be rescanned. Empty roots are removed deepest-first and only if still empty.
Configuration rewrites run before scripts and plugins so references disappear
before their targets.

Git worktree cleanup adds further checks at apply time: the snapshot must be
unchanged, `git status` must contain no tracked, untracked, or ignored changes,
and Git must still register the worktree.

Tweaks are never selected by `scan` or `clean`. A named tweak may create a
configuration file only when it was absent during inspection, and fails if the
target appears before apply. Existing files retain the same hash guard as
cleanup rewrites.

## Adding a Provider

Implement [`Provider`](./src/provider.rs), keep the product detector under
`src/providers/<product>`, and register it in the CLI. Detection returns
findings and generic actions; it does not mutate the filesystem.

A provider owns:

- product paths and environment overrides;
- ownership markers and provenance parsing;
- structured mutation logic and explanations;
- the choice between automatic, review-required, and informational findings.

The engine owns action filtering, ordering, concurrency guards, execution, and
reporting. Adding a provider should not add product conditions to the engine or
operations modules.

Add preference recipes through [`Tweak`](./src/tweak.rs) and register them in
the CLI. A tweak must report whether its desired state is satisfied, needs a
change, or is blocked; it must not appear in the automatic cleanup plan.

## Verification

```console
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```


## Author

Clean the Agent © Wibus, Released under MIT. Created on Sep 15, 2026

> [Personal Website](http://wibus.ren/) · [Blog](https://blog.wibus.ren/) · GitHub [@wibus-wee](https://github.com/wibus-wee/) · Telegram [@wibus✪](https://t.me/wibus_wee)
