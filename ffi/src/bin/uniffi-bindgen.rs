//! Generates the Kotlin and Swift bindings: `cargo run -p khmer-ffi --features bindgen
//! --bin uniffi-bindgen -- generate --library <libkhmer_ffi> --language kotlin --out-dir <dir>`.

fn main() {
    uniffi::uniffi_bindgen_main();
}
