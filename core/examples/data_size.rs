//! Compile an exported directory and report the size and load time:
//! `cargo run --release -p khmer-core --example data_size -- data/build`
use std::path::Path;
use std::time::Instant;

fn main() {
    let directory = std::env::args().nth(1).expect("usage: data_size DIRECTORY");
    let started = Instant::now();
    let bytes = khmer_core::data::compile(Path::new(&directory)).expect("compile");
    let compiled = started.elapsed();
    let size = bytes.len();
    let started = Instant::now();
    let data = khmer_core::data::Data::from_bytes(bytes).expect("load");
    println!(
        "{} words, {:.1} MB, compiled in {:.2} s, loaded in {:.1} ms",
        data.len(),
        size as f64 / 1e6,
        compiled.as_secs_f64(),
        started.elapsed().as_secs_f64() * 1e3
    );
}
