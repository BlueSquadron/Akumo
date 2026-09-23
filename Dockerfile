# Akumo — container image (ADR-0003: single static binary).
#
# Multi-stage: build a musl static binary on Alpine, then ship it on a minimal Alpine runtime that
# carries CA certificates (needed for the AWS provider's TLS). The result is a small, self-contained
# image that runs the same `akumo` binary distributed standalone.

FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig
WORKDIR /src
COPY code/ ./code/
WORKDIR /src/code
# Reproducible-ish release build (opt-level=z, lto, strip, panic=abort — see code/Cargo.toml).
RUN cargo build --release --bin akumo

FROM alpine:3 AS runtime
RUN apk add --no-cache ca-certificates && adduser -D -u 10001 akumo
COPY --from=builder /src/code/target/release/akumo /usr/local/bin/akumo
# Engagement state is written under /work; mount a volume there to persist ledgers.
WORKDIR /work
USER akumo
ENTRYPOINT ["/usr/local/bin/akumo"]
CMD ["--help"]
