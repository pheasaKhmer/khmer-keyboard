//! The core must convert and suggest exactly like khmer-engine on the exported sample.

use std::path::PathBuf;

use khmer_core::{Data, Engine, data, tsv};

fn sample() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/sample")
}

fn engine() -> Engine {
    Engine::new(Data::from_bytes(data::compile(&sample()).unwrap()).unwrap())
}

fn report(mismatches: &[String], total: usize) {
    assert!(
        mismatches.is_empty(),
        "{} of {total} differ from the engine:\n{}",
        mismatches.len(),
        mismatches[..mismatches.len().min(15)].join("\n")
    );
}

#[test]
fn conversions_match_the_engine() {
    let engine = engine();
    let rows = tsv::read(&sample().join("golden/convert.tsv")).unwrap();
    let mut mismatches = Vec::new();
    for row in &rows {
        let result = engine.analyze(&row[0], 5);
        let alternatives = result.alternatives.join(" | ");
        if result.text != row[1] || alternatives != row[2] {
            mismatches.push(format!(
                "{:?}\n  want {} | {}\n  got  {} | {}",
                row[0], row[1], row[2], result.text, alternatives
            ));
        }
    }
    report(&mismatches, rows.len());
}

#[test]
fn suggestions_match_the_engine() {
    let engine = engine();
    let rows = tsv::read(&sample().join("golden/suggest.tsv")).unwrap();
    let mut mismatches = Vec::new();
    for row in &rows {
        let got = engine
            .suggest(&row[0], 5)
            .iter()
            .map(|s| format!("{}|{}|{}|{}", s.text, s.origin.name(), s.start, s.end))
            .collect::<Vec<_>>()
            .join(" ; ");
        if got != row[1] {
            mismatches.push(format!("{:?}\n  want {}\n  got  {}", row[0], row[1], got));
        }
    }
    report(&mismatches, rows.len());
}
