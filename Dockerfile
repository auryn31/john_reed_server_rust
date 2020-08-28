FROM rustlang/rust:nightly as builder

# create a new empty shell project
RUN USER=root cargo new --bin john-reed-server
WORKDIR /john-reed-server


# copy over your manifests
COPY ./server/Cargo.lock ./Cargo.lock
COPY ./server/Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./server/src ./src
COPY ./server/build.rs ./build.rs

# build for release
RUN rm ./target/release/deps/john_reed_server*
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /john-reed-server/target/release/john-reed-server .
CMD ["./john-reed-server"]