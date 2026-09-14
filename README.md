# Clean Any

Clean Any is a cross-platform Rust CLI that detects and removes artifacts and
configuration mutations left by developer tools. Orca is the reference
provider. Product knowledge stays behind a small provider trait; the shared
engine only scans, plans, and applies typed cleanup actions.

The workflow is deliberately staged:

```text
detect -> inspect -> classify and explain -> plan -> apply
```

Detection is read-only. `clean` is a dry run unless both `--apply` and `--yes`
are supplied.

## Project Map

| Area | Location | Responsibility |
| --- | --- | --- |
| CLI | [`src/main.rs`](./src/main.rs) | Select providers and roots, render reports, and enforce explicit apply confirmation. |
| Provider contract | [`src/provider.rs`](./src/provider.rs) | Define the read-only `Provider` trait, platform context, and local/WSL/SSH home scopes. |
| Finding model | [`src/model.rs`](./src/model.rs) | Represent ownership, safety, evidence, scope, snapshots, plans, and typed actions. |
| Engine | [`src/engine.rs`](./src/engine.rs) | Run providers, exclude review-gated actions by default, order dependencies, and report results. |
| Operations | [`src/operations.rs`](./src/operations.rs) | Apply hash-guarded file rewrites, snapshot-guarded removals, and guarded Git worktree removal. |
| Orca composition | [`src/providers/orca/mod.rs`](./src/providers/orca/mod.rs) | Compose independent Orca detectors without exposing Orca paths to the engine. |
| Orca detectors | [`src/providers/orca`](./src/providers/orca) | Own config, state, worktree, runtime, integration, trust, skill, path, and evidence rules. |

## Usage

Build, scan, and inspect the automatic cleanup plan:

```console
cargo build
cargo run -- scan
cargo run -- clean
```

Apply only the automatic plan after reviewing it:

```console
cargo run -- clean --apply --yes
```

Consequential state, including live worktrees, backups, terminal history,
diverged copies, and live trust entries, requires a second opt-in:

```console
cargo run -- clean --include-review
cargo run -- clean --include-review --apply --yes
```

Use `--json` for machine-readable output. Explicit roots support fixtures,
mounted homes, WSL distributions, and mounted SSH filesystems:

```console
cargo run -- --home /mounted/home scan --json
cargo run -- --orca-data-dir /var/lib/orca scan
cargo run -- --workspace-root /workspaces clean
cargo run -- --wsl-home /mnt/wsl/home/alice scan
cargo run -- --remote-home /mnt/ssh/server/home/alice scan
cargo run -- --stale-days 14 clean
```

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
| Skills | Uses Orca skill receipts plus canonical `.agents/skills` topology to find provider links and fallback copies across Claude, Cursor, Gemini, Factory, Continue, Trae, Grok, and Augment roots. Diverged or unreceipted copies are not automatic. |
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

## Verification

```console
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```
