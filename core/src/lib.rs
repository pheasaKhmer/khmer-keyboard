//! Conversion between romanized Khmer and Khmer script, shared by the iOS and Android
//! keyboards.

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert_ne!(super::VERSION, "");
    }
}
