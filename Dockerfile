FROM lukemathwalker/cargo-chef:latest-rust-1.82-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt update && \
    apt upgrade -y && \
    apt install ca-certificates openssl libpq5 -y && \
    apt clean

COPY --from=builder /app/target/release/plant-swap /usr/local/bin/plantswap

WORKDIR /usr/local/plantswap
COPY .env.prod .env
COPY assets assets

EXPOSE 3000
CMD ["plantswap"]
