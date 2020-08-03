FROM rustlang/rust:nightly as builder
COPY ./server/ .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/john-reed-server /usr/local/bin/john-reed-server
CMD ["john-reed-server"]