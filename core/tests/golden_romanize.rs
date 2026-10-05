//! The core must romanize like khmer-engine, including words the lexicon does not know.

use std::path::PathBuf;

use khmer_core::romanize::romanize;
use khmer_core::{Data, Style, data, tsv};

#[test]
fn romanizations_match_the_engine() {
    let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/sample");
    let data = Data::from_bytes(data::compile(&sample).unwrap()).unwrap();
    let rows = tsv::read(&sample.join("golden/romanize.tsv")).unwrap();
    let mismatches: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let style = if row[1] == "chat" {
                Style::Chat
            } else {
                Style::Ungegn
            };
            let got = romanize(&data, &row[0], style);
            (got != row[2])
                .then(|| format!("{:?} {}: want {:?} got {:?}", row[0], row[1], row[2], got))
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} of {} differ:\n{}",
        mismatches.len(),
        rows.len(),
        mismatches[..mismatches.len().min(15)].join("\n")
    );
}
