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
# Content-hash the wasm bundle's output name so every build with changed
# JS/wasm gets a brand new /pkg/webcaldav-<hash>.* URL — the actual
# cache-busting mechanism `cargo leptos build`'s `hash-files = true` would
# give us, since this hand-rolled build (same approach as notion-ical-sync's
# Dockerfile) bypasses cargo-leptos entirely and never produces one on its
# own. LEPTOS_OUTPUT_NAME is read at container startup (see ENTRYPOINT
# below) so the SSR shell's <script src> always points at this build's hash.
RUN HASH=$(sha256sum target/wasm32-unknown-unknown/release/webcaldav.wasm | cut -c1-12) && \
    echo "webcaldav-$HASH" > /app/output-name.txt && \
    wasm-bindgen target/wasm32-unknown-unknown/release/webcaldav.wasm --out-dir pkg --out-name "webcaldav-$HASH" --target web --no-typescript

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/webcaldav /usr/local/bin/webcaldav
COPY --from=wasm-builder /app/pkg /pkg
COPY --from=wasm-builder /app/output-name.txt /output-name.txt
WORKDIR /
EXPOSE 8080
ENTRYPOINT ["/bin/sh", "-c", "export LEPTOS_OUTPUT_NAME=$(cat /output-name.txt) && exec webcaldav"]
