build-tara-release:
    cargo build --bin=tara --release

run-tara:
    cargo run --bin=tara daemon

build-web-config-release:
    cargo leptos build --release

build-all-release: build-tara-release build-web-config-release

watch-web-config:
    @echo "opening at http://127.0.0.1:3000"
    cargo leptos watch

setup:
    cargo install cargo-leptos
