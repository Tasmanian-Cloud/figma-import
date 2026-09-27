# figma-import

[![CI](https://github.com/Ptyktos/figma-import/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Ptyktos/figma-import/actions/workflows/ci.yml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/Ptyktos/figma-import/badge)](https://scorecard.dev/viewer/?uri=github.com/Ptyktos/figma-import)

Import a binary Figma `.fig` export and receive a normalized `PenDocument` JSON tree. This is a small HTTP service built around the [op-figma parser](https://github.com/ZSeven-W/openpencil/tree/main/crates/op-figma), intended for trusted, server-side import pipelines.

It is a one-way import. It does not edit or publish Figma files, call Figma APIs, or provide a browser editor.

## What it returns

`POST /import` accepts a multipart upload with a required `file` field and an optional `name` field. A successful response contains the parser's `PenDocument`, parser warnings, and a summary of pages, node types, text, image references, and unsupported features.

```sh
curl -sS http://localhost:3000/import \
  -F 'file=@design.fig' \
  -F 'name=Landing page' | jq .
```

`GET /health` returns `{"ok":true}`. Invalid, unsupported, or malformed files return a JSON error with an appropriate HTTP status. The upload limit is 512 MiB.

## Fidelity

The importer preserves page and node hierarchy, names, plain text, geometry, common node types, and image references when the parser can resolve them. Styled text is reduced to plain text. Figma variables, exact auto-layout semantics, component overrides, prototype interactions, and some effects and vector details are not fully represented. The response includes the parser's warnings and an `unsupportedFeatures` summary; inspect these before relying on an import.

The included test fixture is synthetic. It exercises the binary parsing path but does not establish fidelity for every real Figma feature. Validate representative exports from your own workflow before production use.

## Run locally

Requires Rust 1.94 or newer (Rust 2024 edition).

```sh
cargo run --locked --bin figma-import
```

The service uses port `3000` by default; set `PORT` to change it. It binds to `0.0.0.0`. There is no authentication, and permissive CORS is not access control. Run it behind an authenticated service tier on a trusted private network, and restrict ingress. See [SECURITY.md](SECURITY.md).

To run it in Docker:

```sh
docker build -t figma-import .
docker run --rm -p 3000:3000 figma-import
```

## Releases

Pushing a version tag such as `v0.1.0` runs the release workflow. The tag must match the package version in `Cargo.toml`; the workflow builds a Linux x86_64 binary and attaches a tarball and SHA-256 checksum to the GitHub Release. Review CI and supply-chain results before tagging.

## Development

```sh
cargo build --locked
cargo test --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidance. Security issues should be reported privately using [SECURITY.md](SECURITY.md).

## Security and dependencies

Pull requests run formatting, build, test, and Clippy checks. Scheduled supply-chain checks run RustSec audit, cargo-deny, and CycloneDX SBOM generation. OpenSSF Scorecard publishes a security-practices report when the repository is public and its workflow is enabled. Dependabot proposes Cargo and GitHub Actions updates.

These automated checks do not replace review or deployment controls. Review [SECURITY.md](SECURITY.md) and the service's network and resource limits before deploying.

## License

MIT. See [LICENSE](LICENSE).
