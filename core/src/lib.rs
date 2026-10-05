//! Conversion between romanized Khmer and Khmer script, shared by the iOS and Android
//! keyboards.
//!
//! The core ports khmer-engine's matching and search. Everything slow (romanizing every
//! word, building the index, tuning) is done by the engine and exported (see `data/`).

pub mod data;
pub mod fuzzy;
pub mod keys;
pub mod model;
pub mod score;
pub mod tsv;

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert_ne!(super::VERSION, "");
    }
}
