# Contributing

Thanks for considering a contribution. Issues and pull requests are welcome for bug fixes, parser fidelity, service reliability, documentation, and security.

## Before opening a pull request

- Search existing issues and pull requests; describe the user-visible problem and expected behavior.
- Keep changes focused. Include or update a regression case when behavior changes.
- Never include real customer `.fig` files, credentials, access tokens, or private design data in commits or issue attachments. Use synthetic fixtures or obtain permission to share a minimized file.
- For parser changes, state which Figma features and file types were exercised and what remains unsupported.
- For dependency changes, explain why the dependency is needed and keep `Cargo.lock` updated.

## Development setup

The project uses Rust 2024 and requires Rust 1.94 or newer (the pinned `op-figma` dependency sets this floor).

```sh
cargo build --locked
cargo test --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
```

Run the service locally with `cargo run --locked --bin figma-import`. It listens on port 3000 by default; `PORT` changes the port. It binds to all interfaces, so keep local testing on a trusted machine and do not expose it to an untrusted network.

## Pull requests

Open a pull request against `main`. Include a concise summary, the commands run and their outcomes, any security or compatibility impact, and screenshots or sample output when they clarify a change. CI must pass before merge. Do not include generated build output or unrelated formatting changes.

## Security reports

Do not report vulnerabilities in public issues or pull requests. Follow [SECURITY.md](SECURITY.md) for private reporting.

## License

By contributing, you agree that your contribution is offered under the repository's MIT license.
