# Security Policy

Clean the Agent deletes files and rewrites configuration, so false ownership,
symlink traversal, path confusion, and changed-after-scan behavior are security
issues even when they do not expose data.

## Supported Versions

Security fixes are provided for the latest release and the current `main`
branch. Older releases may be asked to upgrade before a fix is evaluated.

## Reporting a Vulnerability

Do not open a public issue for a vulnerability or include sensitive local data
in a report. Use
[GitHub private vulnerability reporting](https://github.com/wibus-wee/clean-the-agent/security/advisories/new)
and include:

- the affected version, operating system, and interface;
- the exact command or workflow that reaches the issue;
- the expected safety boundary and the observed result;
- a minimal, synthetic reproduction when possible;
- whether files were removed, configuration was changed, or secrets were
  exposed.

You should receive an acknowledgement within seven days. Maintainers will
confirm the impact, coordinate a fix and disclosure, and credit reporters who
want attribution. Please allow a reasonable remediation window before public
disclosure.

General residue discoveries, unsupported agent products, and cleanup false
positives that do not require coordinated disclosure belong in the public issue
templates.
