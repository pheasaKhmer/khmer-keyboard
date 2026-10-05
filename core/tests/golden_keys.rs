//! The core's matching keys must equal khmer-engine's for every exported input.

use std::path::PathBuf;

use khmer_core::keys::key;
use khmer_core::tsv;

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../data/sample/golden")
        .join(name)
}

#[test]
fn keys_match_the_engine() {
    let rows = tsv::read(&golden("keys.tsv")).unwrap();
    assert!(rows.len() > 10_000);
    let mismatches: Vec<_> = rows
        .iter()
        .filter(|row| key(&row[0], row[1] == "1") != row[2])
        .map(|row| {
            format!(
                "{:?} final={} want {:?} got {:?}",
                row[0],
                row[1],
                row[2],
                key(&row[0], row[1] == "1")
            )
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n{}",
        mismatches.len(),
        mismatches[..mismatches.len().min(20)].join("\n")
    );
}
