//#NOTE This is a client-only binary, primarily used for Wasm
fn main() {
    // When building for WASM, print panics to the browser console
    // #[cfg(target_arch = "wasm32")]
    // console_error_panic_hook::set_once();

    client::run();
}
