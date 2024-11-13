# Multistage build
FROM rust:1.82 AS builder

WORKDIR /usr/app

# You should provide this file with the context of the entire project to allow it to build.
COPY . .
RUN apt-get update && apt-get install -y build-essential libopenblas-dev

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/app
RUN apt-get update && apt-get install -y libopenblas0 libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/app/target/release/server .

EXPOSE 3000
CMD ["/usr/app/server"]
