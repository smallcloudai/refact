# This dockerfile can be used to compile refact-lsp for development purposes, 
# for example, to get refact-lsp to bind into docker containers to start threads in containers

FROM lukemathwalker/cargo-chef:latest-rust-alpine3.21 AS chef

FROM chef AS planner
WORKDIR /refact-lsp
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /refact-lsp
COPY --from=planner /refact-lsp/recipe.json recipe.json

RUN apk add --no-cache \
    build-base \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    zlib-static

RUN cargo chef cook --recipe-path recipe.json

COPY . .

RUN cargo build

RUN mkdir -p /output && cp target/debug/refact-lsp /output/