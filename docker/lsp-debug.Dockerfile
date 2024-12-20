# This dockerfile can be used to compile refact-lsp for development purposes, 
# for example, to get refact-lsp to bind into docker containers to start threads in containers

FROM alpine:3.18

RUN apk add --no-cache \
    build-base \
    curl \
    git \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    protobuf-dev \
    zlib-static

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /refact-lsp

COPY Cargo.toml build.rs ./

RUN mkdir src && echo 'fn main() { println!("Dummy main to satisfy Cargo"); }' > src/main.rs

RUN cargo fetch
RUN cargo check

RUN rm -rf src

COPY . .

RUN cargo build

RUN mkdir -p /output && cp target/debug/refact-lsp /output/

