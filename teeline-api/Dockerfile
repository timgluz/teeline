FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p teeline-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/teeline-api /usr/local/bin/teeline-api
ENV PORT=8080
EXPOSE 8080
CMD ["/usr/local/bin/teeline-api"]
