FROM rust:latest as builder

WORKDIR /src
COPY . .
RUN cargo install --path . --root /app

FROM debian:stretch-slim
RUN apt-get update && apt-get install -y openssl ca-certificates dumb-init
COPY --from=builder /app/bin/tabl-cf .

ENTRYPOINT ["dumb-init", "--"]
CMD ["/tabl-cf"]
