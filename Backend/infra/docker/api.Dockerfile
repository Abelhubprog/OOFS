FROM rust:1.79 as builder
WORKDIR /app
COPY . .
RUN cargo build -p api --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
ENV RUST_LOG=info
EXPOSE 8080
CMD ["/usr/local/bin/api"]
