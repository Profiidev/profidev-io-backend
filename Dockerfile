ARG BINARY_NAME_DEFAULT=profidev-io-backend
ARG TARGET=x86_64-unknown-linux-musl

FROM clux/muslrust:stable as chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder

ARG BINARY_NAME_DEFAULT
ARG TARGET
ENV BINARY_NAME=$BINARY_NAME_DEFAULT
ENV TARGET=$TARGET

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target $TARGET --recipe-path recipe.json
COPY . .
RUN cargo build --release --target $TARGET

RUN mkdir -p /build-out
RUN set -x && cp target/x86_64-unknown-linux-musl/release/$BINARY_NAME /build-out/
RUN mkdir /cloud

FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

ARG BINARY_NAME_DEFAULT
ENV BINARY_NAME=$BINARY_NAME_DEFAULT

ENV RUST_LOG="error,$BINARY_NAME=info"
COPY --from=builder /build-out/$BINARY_NAME /

COPY --from=builder /cloud /

CMD ["/profidev-io-backend"]