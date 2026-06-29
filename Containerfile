# ------------------------------
# Stage 1. Build an app
# ------------------------------
FROM rust:1.96.0 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

# ------------------------------
# Stage 2. Build for runtime
# ------------------------------
FROM dhi.io/debian-base:trixie

ARG GIT_REVISION
ARG BUILD_DATE
ARG VERSION

LABEL org.opencontainers.image.title="fauxrest" \
       org.opencontainers.image.description="Pseudo-REST Static API Generator" \
       org.opencontainers.image.url="https://tamada.github.io/fauxrest" \
       org.opencontainers.image.source="https://github.com/tamada/fauxrest" \
       org.opencontainers.image.version=${VERSION} \
       org.opencontainers.image.revision=${GIT_REVISION} \
       org.opencontainers.image.created=${BUILD_DATE} \
       org.opencontainers.image.licenses="MIT"

COPY --from=builder /app/target/release/fauxrest /app/fauxrest
WORKDIR /opt

ENTRYPOINT [ "/app/fauxrest" ]
CMD [ "-h" ]
