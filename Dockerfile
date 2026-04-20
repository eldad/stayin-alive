# ---------- build stage ----------
FROM rust:1.86-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies by building a dummy project first.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
COPY build.rs ./
COPY proto ./proto
RUN cargo build --release && rm -rf src target/release/.fingerprint/stayin-alive-*

# Build the real binary.
COPY src ./src
RUN cargo build --release

# ---------- runtime stage ----------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/stayin-alive /usr/local/bin/stayin-alive

EXPOSE 3000 50051

ENTRYPOINT ["stayin-alive"]
