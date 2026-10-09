# Windows update discovery fix

> Build 3.2 update: this implementation is now shared with Linux and macOS. The Windows-only boundaries described below record the original redesign phase; see `RELEASE_NOTES_3.2.md` for the current port.

The reported failure was reproduced as GitHub REST API HTTP 403 with `API rate limit exceeded`. Because all app checks used that same unauthenticated API quota, every card showed an error at once.

## Changes

- Windows uses the official public release page and published asset list if REST discovery is unavailable.
- Successful metadata is cached for five minutes across restarts. API rate-limit reset/retry headers are respected across restarts, and concurrent discovery is serialized.
- The App Manager ignores overlapping check-all requests.
- Download candidates remain restricted to the official repository and correct operating-system package. Existing SHA-256 verification remains mandatory.
- Master Suite has an **Allow prerelease updates** switch in **Settings → Updates**. It applies to both automatic and manual checks. New Windows preferences default to stable-only; saved preferences retain their value.
- Version comparison supports alpha/beta/release-candidate suffixes and ignores build metadata when deciding whether a version is newer.
- A newer release missing its Windows installer produces an actionable message rather than a false up-to-date result.

The public feed fallback considers its ten recent versioned releases. Published releases should use semantic version tags such as `v3.2.0` or `v3.2.0-beta.1`; drafts and `untagged-*` tags are not update versions. Stable-only fallback uses GitHub's latest stable release redirect.

The fix is selected only for Windows. Linux and macOS keep their previous implementation until porting is requested. No GitHub account credentials are read or distributed by the updater.

## Evidence

On 9 October 2026, the official public listings for all twelve Craft apps contained their expected Windows x64 portable ZIP and `SHA256SUMS.txt`, despite the REST quota error. The suite installer also returned an exact content length. The Windows executable and setup are rebuilt locally; no release was pushed or published.
