# PowerBaby Gender Reveal (BevyEngine + Naia Networking) 

WIP Multiplayer soccer penalty kick game

## Getting Started

[Install Rust](https://www.rust-lang.org/tools/install)

If you have [Just](https://github.com/casey/just) installed, see the justfile for pipeline shortcuts.  Otherwise:

Install WASM toolchain / helpers
```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli wasm-server-runner
```

For Desktop
```sh
cargo build --release

# in terminal 1
target/release/powerbaby server

# in 'n' terminals
target/release/powerbaby client
```

For Web Wasm + Local Server
```sh
cargo build --release

# in terminal 1
target/release/powerbaby server

# in another terminal (requires wasm-server-runner to be installed)
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --profile wasm-release --bin pbc --target wasm32-unknown-unknown
# navigate to the loaded webserver after compilation
```

## Roadmap
🚧 = In Progress
⛑ = In Testing 
🚀 = Shipped!

| Feature | Status |
| ------- | :------: |
| Ball Physics | 🚀 |
| Dumb Goalie | 🚀 |
| WASM | 🚀 |
| Touch Controls | 🚀 |
| Realtime Multiplayer | ⛑ |
| Leaderboard / Stat Tracking | 🚧 |
| Goalie IQ++ | 🚧 |
| Skybox | 🚧 |
| UI | 🚧 |
| Graphics | 🚧 |
| Sound Effects / Music| 🚧 |
