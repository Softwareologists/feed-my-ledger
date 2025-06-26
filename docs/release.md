# Release Process

Releases are published automatically when a Git tag is pushed that matches `v*.*.*` or `v*.*.*-*`.

Tags containing a hyphen (e.g. `v1.2.0-beta.1`) are treated as pre-releases. The GitHub workflow sets the
`prerelease` flag when uploading the release, so these versions appear as pre-releases on GitHub.
