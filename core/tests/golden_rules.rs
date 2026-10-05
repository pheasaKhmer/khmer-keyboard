//! The core's rule-based romanizer must spell every exported word exactly like
//! khmer-engine's `romanize_word`, in both styles.

use std::path::PathBuf;

use khmer_core::rules::romanize_word;
use khmer_core::{Style, tsv};

#[test]
fn rule_based_romanizations_match_the_engine() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/sample/golden/rules.tsv");
    let rows = tsv::read(&path).unwrap();
    assert!(rows.len() > 6000);
    let mismatches: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let style = if row[1] == "chat" {
                Style::Chat
            } else {
                Style::Ungegn
            };
            let got = romanize_word(&row[0], style);
            (got != row[2])
                .then(|| format!("{:?} {}: want {:?} got {:?}", row[0], row[1], row[2], got))
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} of {} differ:\n{}",
        mismatches.len(),
        rows.len(),
        mismatches[..mismatches.len().min(20)].join("\n")
    );
}
