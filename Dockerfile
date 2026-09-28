# Two-stage build, mirroring the sibling Ptyktos Rust services
# (ai-gateway/pii-svc): a full `rust:*` builder stage, a distroless
# runtime that ships only the compiled binary.
#
# Builder floor is 1.94, not this org's usual 1.88: op-figma's own
# workspace (ZSeven-W/openpencil, vendored via git dependency — see
# Cargo.toml) declares `rust-version = "1.94"`, and cargo hard-errors
# building it on an older toolchain. Bump this if upstream's floor moves.
#
# Both base images are pinned by digest, not floating tag — per the OpenSSF
# Pinned-Dependencies check (scorecard.dev). When bumping rust/trixie or
# distroless, re-resolve the digest and update both lines.
FROM rust:1.96-trixie@sha256:1f0dbad1df66647807e6952d1db85d0b2bda7606cb2139d82517e4f009967376 AS builder
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
COPY tests ./tests

RUN cargo build --release --bin figma-import

FROM gcr.io/distroless/cc-debian13:debug-nonroot@sha256:f525a9a37aed3e8a848f46cfe055999782d66ed797e9e2886928c8caaaa4fc52 AS runtime
COPY --from=builder /build/target/release/figma-import /usr/local/bin/figma-import

ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/figma-import"]
