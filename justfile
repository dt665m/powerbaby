set dotenv-load
alias b := build 
alias s := server
alias c := client

# Path and Variables
ORG := "dt665m"
PROJECT := "powerbaby"
REPO := "https://github.com" / ORG / PROJECT
ROOT_DIR := justfile_directory()
OUTPUT_DIR := ROOT_DIR / "target"
SEM_VER := `awk -F' = ' '$1=="version"{print $2;exit;}' ./Cargo.toml`

default:
    @just --choose

semver:
	@echo {{SEM_VER}}

build:
    cargo build --release

build-wasm:
    cargo build --profile wasm-release --target wasm32-unknown-unknown --bin pbc 
    wasm-bindgen --out-dir ./target/out/ --target web ./target/wasm32-unknown-unknown/wasm-release/pbc.wasm

client:
    target/release/powerbaby client

server:
    target/release/powerbaby server

single:
    target/release/powerbaby single

wasm:
    CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --profile wasm-release --bin pbc --target wasm32-unknown-unknown

#NOTE zld did not install using `brew install zld` due to missing xcodebuild/xcode-select
