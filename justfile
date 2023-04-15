set dotenv-load
alias b := build 
alias s := server
alias c := client
alias rw := run-wasm-build

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
    cp website/public/game.html target/out
    cp -r assets target/out

build-wasm-single:
    cargo build --profile wasm-release --target wasm32-unknown-unknown --bin pbcs
    wasm-bindgen --out-dir ./target/out/ --target web ./target/wasm32-unknown-unknown/wasm-release/pbcs.wasm
    cp website/public/game-single.html target/out
    cp -r assets target/out

run-wasm-build: build-wasm
    python3 -m http.server --directory target/out

client:
    target/release/powerbaby client

server:
    RUST_BACKTRACE=true target/release/powerbaby server

single:
    target/release/powerbaby single

wasm:
    CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --profile wasm-release --bin pbc --target wasm32-unknown-unknown

wasm-opt:
    wasm-opt -O -ol 100 -s 100 -o target/out/pbc_bg-opt.wasm target/out/pbc_bg.wasm
    mv target/out/pbc_bg.wasm target/out/pbc_bg_original.wasm
    mv target/out/pbc_bg-opt.wasm target/out/pbc_bg.wasm

wasm-opt-single:
    wasm-opt -O -ol 100 -s 100 -o target/out/pbcs_bg-opt.wasm target/out/pbcs_bg.wasm
    mv target/out/pbcs_bg.wasm target/out/pbcs_bg_original.wasm
    mv target/out/pbcs_bg-opt.wasm target/out/pbcs_bg.wasm

build-website: build-wasm
    cd website && yarn build && cp ../target/out/* build

publish-website: build-website
    cd website && yarn publish:site

#NOTE zld did not install using `brew install zld` due to missing xcodebuild/xcode-select
