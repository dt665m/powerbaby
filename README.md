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

# in 'n' terminals (requires wasm-server-runner to be installed)
# navigate to the loaded webserver after compilation
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --profile wasm-release --bin pbc --target wasm32-unknown-unknown
```



## Roadmap
🚧 = In Progress
⛑ = In Testing 
🚀 = Shipped!

| Feature | Status |
| ------- | :------: |
| Ball Physics | 🚀 |
| Dumb Goalie | 🚀 |
| WASM | 🚀  |
| Realtime Multiplayer | ⛑  |
| Leaderboard / Stat Tracking | 🚧  |
| Goalie IQ++ | 🚧  |
| UI | 🚧  |
| Graphics | 🚧 |
| Sound Effects / Music| 🚧 |
