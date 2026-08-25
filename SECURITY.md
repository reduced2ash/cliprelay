# Security policy

## Supported versions

Security fixes are applied to the latest published version of ClipRelay.
Pre-release builds are supported only until a newer pre-release is available.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability involving:

- Telegram tokens, sessions, API credentials, or account access
- arbitrary file access or deletion
- source-video modification
- generated-file cleanup escaping the configured export directory
- command execution through a filename or caption
- release signing or update-channel compromise

Use GitHub's private vulnerability reporting for this repository:

1. Open the repository's **Security** tab.
2. Choose **Report a vulnerability**.
3. Include the affected version, operating system, reproduction steps, impact,
   and any safe proof-of-concept material.

You should receive an acknowledgement within seven days. Please allow time for
a fix and coordinated release before public disclosure.

## Handling secrets

ClipRelay uses the operating-system credential store when available. A
permission-restricted local fallback may be used if the credential store is
unavailable. Never attach the fallback file, application database, or raw logs
to a public issue.

Maintainers must store release certificates and service credentials only in an
operating-system keychain or encrypted GitHub Actions secrets. The repository
ignores common credential file formats as a defense in depth measure.
