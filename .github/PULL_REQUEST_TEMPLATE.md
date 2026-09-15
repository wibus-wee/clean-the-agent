## What changed

<!-- Describe the user-visible result and keep this pull request focused. -->

## Evidence and safety boundary

<!-- For cleanup rules: how is ownership proven, what is removable, and what is deliberately retained? -->

## Verification

<!-- List the commands and fixtures used to verify the change. -->

- [ ] Detection remains read-only.
- [ ] Automatic actions remove only a proven, minimal unit.
- [ ] Similar user-owned, malformed, modified, and changed-after-scan state is tested.
- [ ] CLI formatting, clippy, tests, and docs pass.
- [ ] GUI checks pass when shared behavior or the GUI changed.
- [ ] User-facing coverage or behavior is documented.
- [ ] No secrets or personal data are included in fixtures, logs, or screenshots.
