# AGENT Guidelines for rusty-ledger

This file outlines best practices for working with this Rust project.

## Code Style
- Run `cargo fmt` before committing. The project uses Rust edition 2024 and a `max_width` of 100.
- Lint with `cargo clippy` and fix warnings. The repository's `Clippy.toml` sets `avoid-breaking-exported-api = true`.
- Keep functions small and focused. Prefer explicit error handling over `unwrap` in library code.

## Testing
- Execute `cargo test` for all changes. Add unit tests for new functionality when possible.
- Tests and formatting must pass before committing.

## Commit Messages
- Use short, imperative messages (e.g., "Add OAuth tests").
- Reference relevant issues when applicable.

## Pull Requests
- Summarize the purpose of the change and how it was tested.
- Ensure `cargo fmt`, `cargo clippy`, and `cargo test` succeed prior to opening a PR.

For additional contribution details, refer to [CONTRIBUTING.md](CONTRIBUTING.md).
