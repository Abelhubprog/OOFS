FROM rust:1.79 as builder
WORKDIR /app
COPY . .
RUN cargo build -p workers --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/workers /usr/local/bin/workers
ENV RUST_LOG=info
CMD ["/usr/local/bin/workers"]
