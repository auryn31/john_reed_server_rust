FROM rustlang/rust:nightly as builder
COPY ./server/ .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/hello-rocket /usr/local/bin/hello-rocket
CMD ["hello-rocket"]