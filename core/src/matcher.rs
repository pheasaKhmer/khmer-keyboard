//! Find the Khmer words a romanized word could be, ported from khmer-engine's
//! `Matcher.emissions` and `Matcher.completions`.

use std::collections::{HashMap, HashSet};

use crate::data::{Data, Form, Source};
use crate::fuzzy;
use crate::keys::{fold, key};
use crate::model::logprob;
use crate::score::sort_by_score;

/// A candidate word with its emission score and the form that matched best.
#[derive(Clone, Copy, Debug)]
pub struct Emission<'a> {
    pub word: u32,
    pub score: f64,
    pub form: Form<'a>,
}

/// Keeps the first-seen order of words, like the engine's dict, while scores improve.
#[derive(Default)]
struct Best<'a> {
    order: Vec<Emission<'a>>,
    index: HashMap<u32, usize>,
}

impl<'a> Best<'a> {
    fn offer(&mut self, emission: Emission<'a>) {
        match self.index.get(&emission.word) {
            Some(&i) if emission.score > self.order[i].score => self.order[i] = emission,
            Some(_) => {}
            None => {
                self.index.insert(emission.word, self.order.len());
                self.order.push(emission);
            }
        }
    }
}

/// Each candidate word for `text`, in the engine's order, with its emission score: the
/// log of how likely the typed spelling is for the word.
pub fn emissions<'a>(data: &'a Data, text: &str, fuzzy_keys: bool) -> Vec<Emission<'a>> {
    let typed = fold(text);
    let typed_key = key(&typed, true);
    if typed_key.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(Form<'a>, usize)> = data.forms(&typed_key).map(|f| (f, 0)).collect();
    if fuzzy_keys && typed_key.len() >= data.settings.min_fuzzy_key {
        let mut seen: HashSet<String> = HashSet::from([typed_key.clone()]);
        for other in fuzzy::neighbours(&typed_key, &data.alphabet) {
            if !seen.contains(&other) {
                hits.extend(data.forms(&other).map(|f| (f, 1)));
                seen.insert(other);
            }
        }
    }
    let settings = &data.settings;
    let typed_length = typed.chars().count();
    let mut best = Best::default();
    for (form, key_edits) in hits {
        let letters = fuzzy::distance(&typed, form.spelling);
        let longest = typed_length.max(form.spelling.chars().count());
        let mut score = -settings.key_edit * key_edits as f64;
        score -= settings.spelling * letters as f64 / longest as f64;
        match form.source {
            Source::Curated => score += settings.curated,
            Source::Consonants => score -= settings.abbreviation,
            Source::Minor => score -= settings.minor,
            Source::Pronunciation | Source::Spelling | Source::Ungegn => {}
        }
        best.offer(Emission {
            word: form.word,
            score,
            form,
        });
    }
    best.order
}

/// Words whose key starts with the key of `text` and is longer: readings of a word still
/// being typed. Keeps the `limit` words with the best emission and frequency.
pub fn completions<'a>(data: &'a Data, text: &str, limit: usize) -> Vec<Emission<'a>> {
    let settings = &data.settings;
    let prefix = key(text, false);
    if prefix.len() < settings.min_completion_key {
        return Vec::new();
    }
    let mut best = Best::default();
    for (other, packed) in data.keys_with_prefix(&prefix, settings.max_completion_scan) {
        if other == prefix {
            continue;
        }
        let missing = (other.len() - prefix.len()) as f64;
        let score = -settings.completion - settings.missing * missing;
        for form in data.forms_at(packed) {
            best.offer(Emission {
                word: form.word,
                score,
                form,
            });
        }
    }
    let value = |e: &Emission<'_>| e.score + settings.frequency * logprob(data, data.word(e.word));
    let mut ranked = sort_by_score(best.order, value);
    ranked.truncate(limit);
    ranked
}

#[cfg(test)]
mod tests {
    use super::{completions, emissions};
    use crate::data::{Source, sample};

    #[test]
    fn exact_and_curated_matches() {
        let data = sample();
        let words = |text: &str| -> Vec<String> {
            emissions(&data, text, true)
                .iter()
                .map(|e| data.word(e.word).to_owned())
                .collect()
        };
        assert!(words("orkun").contains(&"អរគុណ".to_owned()));
        assert!(words("sousdey").contains(&"សួស្តី".to_owned()));
        let jg = emissions(&data, "jg", true);
        assert!(
            jg.iter()
                .any(|e| e.form.source == Source::Curated && data.word(e.word) == "ចង់")
        );
        assert!(emissions(&data, "123", true).is_empty());
    }

    #[test]
    fn exact_keys_only_when_fuzzy_is_off() {
        let data = sample();
        let fuzzy = emissions(&data, "orkon", true);
        let exact = emissions(&data, "orkon", false);
        assert!(fuzzy.len() > exact.len());
    }

    #[test]
    fn completions_start_with_what_was_typed() {
        let data = sample();
        let found = completions(&data, "orku", 20);
        assert!(found.iter().any(|e| data.word(e.word) == "អរគុណ"));
        assert!(completions(&data, "o", 20).is_empty());
    }
}
