# Contributing to Clean the Agent

Clean the Agent welcomes evidence and cleanup support for any agent product
that leaves files, processes, worktrees, hooks, integrations, or configuration
behind after uninstall. You can help without writing Rust: a reproducible,
redacted residue report is enough to start a provider rule.

## Ways to Contribute

| Contribution | Start here | What makes it useful |
| --- | --- | --- |
| Report agent residue | [Agent residue issue](https://github.com/wibus-wee/clean-the-agent/issues/new?template=agent-residue.yml) | Product version, platform, uninstall method, paths or configuration shapes, and ownership evidence. |
| Report incorrect behavior | [Bug report](https://github.com/wibus-wee/clean-the-agent/issues/new?template=bug-report.yml) | Command, expected result, actual result, and a minimal redacted fixture. |
| Propose a capability | [Feature request](https://github.com/wibus-wee/clean-the-agent/issues/new?template=feature-request.yml) | User problem, safety boundary, and expected workflow. |
| Implement a provider | Open a pull request | Read-only detection, conservative classification, guarded actions, and tests. |
| Improve the project | Open a pull request | Tests, explanations, accessibility, packaging, platform support, or documentation. |

Never post tokens, credentials, private prompts, terminal history, customer
names, private repository URLs, or complete personal configuration files.
Replace identifying values consistently so relationships remain visible.

## Development Setup

The CLI requires Rust 1.88 or later. From the repository root:

```console
cargo build --locked
cargo test --locked --all-targets --all-features
```

The native GUI requires macOS 14 or later, Rust 1.90 or later, Bun 1.4 or
later, and the Xcode Command Line Tools:

```console
cd gui
bun ci
bun run dev
```

The root CLI and `gui` are separate Cargo workspaces. Run Cargo commands against
both manifests when a shared-library change can affect the GUI.

## Provider Contract

A provider owns product-specific paths, environment overrides, provenance,
structured parsing, explanations, and safety classification. The shared engine
owns filtering, ordering, concurrency guards, execution, and reporting.

Keep the following boundaries:

1. Detection is read-only. A scan must not create, rewrite, move, or remove
   files and must not stop processes.
2. Attribute a target using product-owned roots, exact generated forms,
   ownership records, or independently verifiable provenance. A matching name
   alone is insufficient.
3. Remove the smallest proven unit. Structurally edit shared configuration
   instead of deleting a whole file.
4. Classify state as `automatic` only when both ownership and the removable
   boundary are proven. Use `review_required` when useful work may remain, and
   `informational` when no conservative action exists.
5. Preserve user work, history, credentials, unknown files, modified copies,
   and newer formats the detector does not understand.
6. Use scan-time hashes or path snapshots for actions. If the target changes,
   fail closed and require another scan.
7. Do not follow symlinks during recursive deletion. Treat links and their
   targets as separate ownership decisions.
8. Explain why each finding belongs to the product and why its safety level is
   appropriate. A user should not need to read the implementation before
   confirming a plan.

Place a provider under `src/providers/<product>` and implement the trait in
[`src/provider.rs`](./src/provider.rs). Return generic findings and actions from
detection; do not add product-specific branches to the engine or operations
modules. Register the provider in both user interfaces when it is ready for
users.

Preference changes belong behind the separate [`Tweak`](./src/tweak.rs)
contract. Tweaks are explicit desired-state recipes and must never enter an
automatic cleanup plan.

## Evidence and Tests

Every automatic cleanup rule should include fixtures that demonstrate:

- the owned form is detected and planned;
- similar user-owned or third-party state is retained;
- malformed, partial, and unknown-newer forms are not removed;
- a target changed after scanning causes the action to fail closed;
- shared configuration retains unrelated keys, comments, ordering, or content
  promised by that format;
- a scan leaves the fixture tree byte-for-byte unchanged;
- relevant local, WSL, mounted remote, and platform-specific paths keep their
  scope labels.

Review-required and informational rules still need positive attribution and
negative tests. Avoid fixtures copied from a real home directory; synthesize the
smallest representative tree and redact all personal data.

## Required Checks

Before opening a pull request, run:

```console
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo fmt --manifest-path gui/Cargo.toml --all -- --check
```

On macOS, also run:

```console
cd gui
bun ci
bun run check
bun run test
bun run build -- --sign -
```

GitHub Actions repeats the CLI build and test suite on macOS, Linux, and Windows,
checks the CLI's declared minimum Rust version, and packages the macOS GUI.
Pull requests from forks receive only read access and do not require secrets.

## Pull Requests

Keep a pull request focused on one product surface or one shared behavior. In
the description, identify the evidence, cleanup boundary, safety class, tests,
and any state deliberately left untouched. Update the README coverage table
when user-visible support changes.

By contributing, you agree that your contribution is licensed under the
repository's [MIT License](./LICENSE) and that you will follow the
[Code of Conduct](./CODE_OF_CONDUCT.md).

## Maintainer Releases

1. Update the version in both [`Cargo.toml`](./Cargo.toml) and
   [`gui/Cargo.toml`](./gui/Cargo.toml), then refresh lockfiles if they change.
2. Merge the version change only after CI passes.
3. Create and push a matching tag, for example `v0.2.0`.
4. The release workflow validates the tag, reruns tests, builds all CLI archives
   and macOS DMGs, verifies ad-hoc signatures, creates `SHA256SUMS.txt`, and
   publishes the GitHub Release.

Prerelease tags such as `v0.2.0-rc.1` create GitHub prereleases. The current
macOS packages are intentionally ad-hoc signed and cannot be notarized without
a Developer ID identity and Apple notarization credentials.
