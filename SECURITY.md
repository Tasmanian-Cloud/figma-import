# Security policy

## Report a vulnerability

Email **security@tasmanian.cloud** to report a suspected vulnerability. Do not open a public issue for an unpatched vulnerability. Include the affected version or commit, impact, and steps to reproduce. The maintainers will acknowledge reports as soon as practical and coordinate a fix and disclosure with the reporter.

Once GitHub private vulnerability reporting is enabled in repository settings, reports can also be submitted through [GitHub Security Advisories](https://github.com/Tasmanian-Cloud/figma-import/security/advisories/new).

## Supported versions

Before the first stable release, security fixes are made against the latest `main` branch. After a stable release, the latest supported release line will be listed here.

## Security-relevant behavior

`figma-import` parses attacker-provided `.fig` bytes. The HTTP service accepts multipart uploads up to 512 MiB and listens on `0.0.0.0` by default. It has no built-in authentication or authorization and should only be reachable from a trusted internal network behind an authenticated application tier. Restrict ingress at the network or proxy layer; do not expose it directly to the public internet.

The parser and service process uploads in memory. Large or malformed files can consume substantial CPU and memory. Set deployment memory, request timeout, and concurrency limits appropriate to your environment. Do not send confidential design files to an instance you do not control.

The service does not call Figma APIs or require Figma credentials. The `/health` endpoint is unauthenticated. CORS is permissive for internal service-to-service use and is not an access-control mechanism.

## Dependency and release security

Rust dependencies are recorded in `Cargo.lock`; CI uses locked builds. Pull requests and scheduled workflows run RustSec advisory checks, cargo-deny policy checks, SBOM generation, and OpenSSF Scorecard. Release artifacts should be built from reviewed tags and accompanied by the workflow's provenance and dependency evidence when configured.
