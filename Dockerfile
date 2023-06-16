FROM rust as builder

RUN USER=root cargo new --bin espy_server
WORKDIR /espy_server

COPY . .

RUN cargo build --release --bin http_server
RUN cargo build --release --bin webhook_handlers

# -----------------------------------------

FROM debian:buster-slim as http_server_image

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /espy_server/target/release/http_server /usr/local/bin/http_server
COPY ./keys.json ./keys.json
COPY ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json

ENV PORT 8080

CMD ["http_server", "--prod-tracing"]

# -----------------------------------------

FROM debian:buster-slim as webhooks_image

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /espy_server/target/release/webhook_handlers /usr/local/bin/webhook_handlers
COPY ./keys.json ./keys.json
COPY ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json

ENV PORT 8080

CMD ["webhook_handlers", "--prod-tracing"]
