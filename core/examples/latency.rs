//! Time `suggest` on every keystroke of 5-word inputs, like khmer-engine's evaluation:
//! `cargo run --release -p khmer-core --example latency -- data/build`
use std::path::Path;
use std::time::Instant;

use khmer_core::{Data, Engine, data, tsv};

fn main() {
    let directory = std::env::args().nth(1).expect("usage: latency DIRECTORY");
    let engine =
        Engine::new(Data::from_bytes(data::compile(Path::new(&directory)).unwrap()).unwrap());
    let inputs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/golden_inputs.txt");
    let words: Vec<String> = tsv::read(&inputs)
        .unwrap()
        .into_iter()
        .take(126)
        .flat_map(|row| row[0].split(' ').map(str::to_owned).collect::<Vec<_>>())
        .collect();
    let mut times = Vec::new();
    for chunk in words.chunks_exact(5).take(20) {
        let text = chunk.join(" ");
        for end in text.char_indices().map(|(i, c)| i + c.len_utf8()) {
            let started = Instant::now();
            std::hint::black_box(engine.suggest(&text[..end], 5));
            times.push(started.elapsed().as_secs_f64() * 1e3);
        }
    }
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    let p95 = times[times.len() * 95 / 100];
    println!(
        "{} keystrokes: median {median:.3} ms, p95 {p95:.3} ms, max {:.3} ms",
        times.len(),
        times[times.len() - 1]
    );
}
