//! Spell romanized words the lexicon does not know, syllable by syllable, ported from
//! khmer-engine's `Transliterator.transliterate`. The syllable table itself is built by
//! the engine and exported.

use crate::data::Data;
use crate::keys::{fold, key};

/// Khmer for `typed` built from known syllables, or `None` if they cannot cover it.
pub fn transliterate(data: &Data, typed: &str) -> Option<String> {
    let letters = fold(typed);
    if letters.is_empty() {
        return None;
    }
    let n = letters.len(); // folded text is ASCII
    let settings = &data.settings;
    let mut best: Vec<Option<(f64, usize, &str)>> = vec![None; n + 1];
    best[0] = Some((0.0, 0, ""));
    for start in 0..n {
        let Some((base, _, _)) = best[start] else {
            continue;
        };
        for end in start + 1..=(start + settings.max_syllable_letters).min(n) {
            let piece_key = key(&letters[start..end], end == n);
            let Some((spelling, logprob)) = data.syllable(&piece_key) else {
                continue;
            };
            let mut score = base + logprob;
            if start > 0 && b"aeiou".contains(&letters.as_bytes()[start]) {
                score -= settings.vowel_onset_cost;
            }
            if best[end].is_none_or(|(current, _, _)| score > current) {
                best[end] = Some((score, start, spelling));
            }
        }
    }
    best[n]?;
    let mut pieces = Vec::new();
    let mut end = n;
    while end > 0 {
        let (_, start, text) = best[end].expect("every step back is reachable");
        pieces.push(text);
        end = start;
    }
    pieces.reverse();
    Some(pieces.concat())
}

#[cfg(test)]
mod tests {
    use super::transliterate;
    use crate::data::sample;

    #[test]
    fn names_are_spelled_from_syllables() {
        let data = sample();
        assert_eq!(transliterate(&data, "sreymom").as_deref(), Some("ស្រីមុំ"));
    }

    #[test]
    fn nothing_to_spell() {
        let data = sample();
        assert_eq!(transliterate(&data, ""), None);
        assert_eq!(transliterate(&data, "123"), None);
    }
}
