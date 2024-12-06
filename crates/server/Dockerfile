# Multistage build
FROM rust:1.82 AS builder

WORKDIR /usr/app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libopenblas-dev \
    liblapack-dev \
    gfortran \
    pkg-config

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/app
RUN apt-get update && \
    apt-get install -y \
    libopenblas0 \
    liblapack3 \
    libssl3 \
    ca-certificates && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/app/target/release/server .

EXPOSE 3000
CMD ["/usr/app/server"]
