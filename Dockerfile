FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --no-default-features --features ssr
COPY . .
RUN cargo build --release --bin webcaldav --no-default-features --features ssr

# Builds the client-side hydration bundle (wasm). Server-rendered shell from
# the `ssr`-featured binary above hydrates into this in the browser — never
# combined into one cargo invocation since `hydrate` pulls in browser-only
# web-sys/DOM/fetch APIs that don't exist on this stage's host target.
FROM chef AS wasm-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli --version 0.2.129 --locked
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --target wasm32-unknown-unknown --no-default-features --features hydrate
COPY . .
RUN cargo build --release --lib --target wasm32-unknown-unknown --no-default-features --features hydrate
RUN wasm-bindgen target/wasm32-unknown-unknown/release/webcaldav.wasm --out-dir pkg --target web --no-typescript

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/webcaldav /usr/local/bin/webcaldav
COPY --from=wasm-builder /app/pkg /pkg
WORKDIR /
EXPOSE 8080
ENTRYPOINT ["webcaldav"]
