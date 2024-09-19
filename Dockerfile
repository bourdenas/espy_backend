FROM rust as builder

RUN USER=root cargo new --bin espy_server
WORKDIR /espy_server

COPY . .

RUN cargo build --release --bin http_server
RUN cargo build --release --bin webhooks_backend
RUN cargo build --release --bin build_timeline

# -----------------------------------------

FROM debian as http_server_image

RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /espy_server/target/release/http_server /usr/local/bin/http_server
COPY ./keys.json ./keys.json
COPY ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json

ENV PORT 8080

CMD ["http_server", "--prod-tracing"]

# -----------------------------------------

FROM debian as webhooks_image

RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /espy_server/target/release/webhooks_backend /usr/local/bin/webhooks_backend
COPY ./keys.json ./keys.json
COPY ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json

ENV PORT 8080

CMD ["webhooks_backend", "--prod-tracing"]

# -----------------------------------------

FROM debian as timeline_image

RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /espy_server/target/release/build_timeline /usr/local/bin/build_timeline
COPY ./keys.json ./keys.json
COPY ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json ./espy-library-firebase-adminsdk-sncpo-3da8ca7f57.json

ENV PORT 8080

CMD ["build_timeline", "--prod-tracing"]
