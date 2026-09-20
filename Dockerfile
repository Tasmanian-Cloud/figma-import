# Two-stage build, mirroring the sibling Ptyktos Rust services
# (ai-gateway/pii-svc): a full `rust:*` builder stage, a distroless
# runtime that ships only the compiled binary.
#
# Builder floor is 1.94, not this org's usual 1.88: op-figma's own
# workspace (ZSeven-W/openpencil, vendored via git dependency — see
# Cargo.toml) declares `rust-version = "1.94"`, and cargo hard-errors
# building it on an older toolchain. Bump this if upstream's floor moves.
FROM rust:1.96-trixie AS builder
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
COPY tests ./tests

RUN cargo build --release --bin figma-import

FROM gcr.io/distroless/cc-debian13:debug-nonroot AS runtime
COPY --from=builder /build/target/release/figma-import /usr/local/bin/figma-import

ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/figma-import"]
