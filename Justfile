# ==================================
# Justfile – workspace Rust WASM
# ==================================

default:
    just --list

# -------- Formatage --------
# fmt:
#     cd src && cargo fmt

# -------- Build --------
build:
    cd src && cargo build

release:
    cd src && cargo build --release

# -------- WASM --------
wasm:
    cd src && cargo build --target wasm32-unknown-unknown

wasm-release:
    cd src && cargo build --release --target wasm32-unknown-unknown

# -------- Tests --------
test:
    cd src && cargo test

# -------- Lint --------
clippy:
    cd src && cargo clippy --all-targets --all-features

# -------- Fix (SCOPÉ PAR CRATE) --------
fix package:
    cd src && cargo fmt
    cd src && (cargo clippy -p {{package}} --fix --allow-dirty --allow-staged || cargo clippy -p {{package}})

# -------- Fix global (fallback) --------
# fix-all:
#     cd src && cargo fmt
#     cd src && (cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged || cargo clippy --all-targets --all-features)

# -------- CI --------
ci:
    cd src && cargo fmt --check
    cd src && cargo clippy --all-targets --all-features -D warnings
    cd src && cargo test
    cd src && cargo build --target wasm32-unknown-unknown

# -------- Setup --------
setup:
    rustup target add wasm32-unknown-unknown
