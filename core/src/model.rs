//! Word probabilities, ported from khmer-engine's `Lexicon.logprob` and
//! `Lexicon.bigram_logprob`.

use crate::data::Data;

/// The corpus count of a lexicon word; 0 for anything else.
fn count(data: &Data, word: &str) -> u32 {
    data.word_id(word)
        .filter(|&id| data.known(id))
        .map_or(0, |id| data.count(id))
}

/// Smoothed log unigram probability (add-one), defined for unknown words too.
pub fn logprob(data: &Data, word: &str) -> f64 {
    let numerator = u64::from(count(data, word)) + 1;
    let denominator = data.total + data.vocabulary + 1;
    (numerator as f64 / denominator as f64).ln()
}

/// Log P(word | previous), interpolated with the unigram probability.
pub fn bigram_logprob(data: &Data, previous: &str, word: &str) -> f64 {
    // The engine takes exp and log of the unigram probability, which can move the last
    // bit; doing the same keeps the scores identical.
    let unigram = logprob(data, word).exp();
    let Some(first) = data.word_id(previous) else {
        return unigram.ln();
    };
    let following = data.following(first);
    if following == 0 {
        return unigram.ln();
    }
    let pairs = data
        .word_id(word)
        .map_or(0, |second| data.bigram(first, second));
    let bigram = f64::from(pairs) / f64::from(following);
    let weight = data.settings.bigram_weight;
    (weight * bigram + (1.0 - weight) * unigram).ln()
}

#[cfg(test)]
mod tests {
    use super::{bigram_logprob, logprob};
    use crate::data::sample;

    #[test]
    fn frequent_words_are_more_likely() {
        let data = sample();
        assert!(logprob(&data, "ទេ") > logprob(&data, "សប្បាយ"));
        assert!(logprob(&data, "សប្បាយ") > logprob(&data, "not a word"));
        assert!(logprob(&data, "not a word").is_finite());
    }

    #[test]
    fn a_seen_pair_beats_the_word_alone() {
        let data = sample();
        assert!(bigram_logprob(&data, "នៅ", "ក្នុង") > logprob(&data, "ក្នុង"));
        // No pairs start with an unknown word: fall back to the word alone.
        let alone = bigram_logprob(&data, "not a word", "ក្នុង");
        assert!((alone - logprob(&data, "ក្នុង")).abs() < 1e-12);
    }
}
