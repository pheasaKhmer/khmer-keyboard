//! The conversion engine: data, matcher and decoder together, ported from khmer-engine's
//! `Engine`.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::data::{Data, Source};
use crate::decode::{Choice, Conversion, Decoder, Origin};
use crate::matcher;
use crate::score::descending;
use crate::transliterate::transliterate;

/// Span readings kept between calls: a keyboard asks on every keystroke, and most spans
/// of the input are the same as last time.
const CHOICE_CACHE_SIZE: usize = 4096;

/// A reading for the word being typed. Replacing characters `start..end` of the input with
/// `text` commits it. Higher scores are better.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub text: String,
    pub score: f64,
    pub start: usize,
    pub end: usize,
    pub origin: Origin,
}

fn origin(source: Source) -> Origin {
    match source {
        Source::Pronunciation => Origin::Pronunciation,
        Source::Spelling => Origin::Spelling,
        Source::Ungegn => Origin::Ungegn,
        Source::Curated => Origin::Curated,
    }
}

pub struct Engine {
    pub data: Data,
    cache: RefCell<HashMap<(String, bool), Vec<Choice>>>,
}

impl Engine {
    pub fn new(data: Data) -> Self {
        Engine {
            data,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Every reading of one span of typed text, with its emission score. A piece of a typed
    /// word (`whole` false) only matches keys exactly and is never English. A single typed
    /// word can also be transliterated or, as a last resort, kept.
    pub fn choices(&self, typed: &str, whole: bool) -> Vec<Choice> {
        let cache_key = (typed.to_owned(), whole);
        if let Some(cached) = self.cache.borrow().get(&cache_key) {
            return cached.clone();
        }
        let choices = self.compute_choices(typed, whole);
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= CHOICE_CACHE_SIZE {
            cache.clear();
        }
        cache.insert(cache_key, choices.clone());
        choices
    }

    fn compute_choices(&self, typed: &str, whole: bool) -> Vec<Choice> {
        let data = &self.data;
        let settings = &data.settings;
        let mut out: Vec<Choice> = matcher::emissions(data, typed, whole)
            .into_iter()
            .map(|e| Choice {
                text: data.word(e.word).to_owned(),
                emission: e.score,
                origin: origin(e.form.source),
                spelling: e.form.spelling.to_owned(),
            })
            .collect();
        let lower = typed.to_lowercase();
        if whole && data.is_english(&lower) {
            out.push(Choice {
                text: typed.to_owned(),
                emission: 0.0,
                origin: Origin::English,
                spelling: lower.clone(),
            });
        }
        if whole && !typed.contains(' ') {
            if let Some(guess) = transliterate(data, typed)
                && !out.iter().any(|c| c.text == guess)
            {
                out.push(Choice {
                    text: guess,
                    emission: settings.fallback_emission,
                    origin: Origin::Fallback,
                    spelling: lower,
                });
            }
            out.push(Choice {
                text: typed.to_owned(),
                emission: settings.typed_emission,
                origin: Origin::Typed,
                spelling: typed.to_owned(),
            });
        }
        out
    }

    fn decoder(&self) -> Decoder<'_, impl FnMut(&str, bool) -> Vec<Choice> + '_> {
        Decoder {
            data: &self.data,
            choices: |typed: &str, whole: bool| self.choices(typed, whole),
        }
    }

    /// The best conversion, the n best alternatives, and ranked readings per span.
    pub fn analyze(&self, text: &str, n: usize) -> Conversion {
        self.decoder().convert(text, n)
    }

    /// The best Khmer conversion of romanized `text`.
    pub fn convert(&self, text: &str) -> String {
        self.analyze(text, 1).text
    }

    /// Ranked readings of the last word of `text`, in the context of the words before it.
    /// While the last word is still being typed (no space or punctuation after it), words
    /// it could be the start of are suggested too.
    pub fn suggest(&self, text: &str, n: usize) -> Vec<Suggestion> {
        let mut decoder = self.decoder();
        let tokens = decoder.convert(text, n).tokens;
        let Some(last) = tokens.last() else {
            return Vec::new();
        };
        let before = tokens.len().checked_sub(2).map(|i| &tokens[i].choices[0]);
        let previous = before.filter(|c| c.is_khmer()).map(|c| c.text.as_str());
        let mut candidates = last.choices.clone();
        if last.end == text.chars().count() {
            for e in matcher::completions(&self.data, &last.typed, 20) {
                candidates.push(Choice {
                    text: self.data.word(e.word).to_owned(),
                    emission: e.score,
                    origin: Origin::Completion,
                    spelling: e.form.spelling.to_owned(),
                });
            }
        }
        let mut best: Vec<Suggestion> = Vec::new();
        for choice in &candidates {
            let score = choice.emission + decoder.language_model(previous, choice);
            let suggestion = Suggestion {
                text: choice.text.clone(),
                score,
                start: last.start,
                end: last.end,
                origin: choice.origin,
            };
            match best.iter_mut().find(|s| s.text == choice.text) {
                Some(existing) if score > existing.score => *existing = suggestion,
                Some(_) => {}
                None => best.push(suggestion),
            }
        }
        best.sort_by(|a, b| descending(a.score, b.score));
        best.truncate(n);
        best
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::Origin;
    use crate::data::sample;

    #[test]
    fn converts_phrases() {
        let engine = Engine::new(sample());
        assert_eq!(engine.convert("sok sabay te"), "សុខសប្បាយទេ");
        assert_eq!(engine.convert("ot mean wifi te?"), "អត់មាន wifi ទេ?");
        assert_eq!(engine.convert("soksabayte"), "សុខសប្បាយទេ");
        assert_eq!(engine.convert(""), "");
    }

    #[test]
    fn suggests_completions_with_the_span_to_replace() {
        let engine = Engine::new(sample());
        let first = &engine.suggest("orku", 5)[0];
        assert_eq!(
            (first.text.as_str(), first.origin, first.start, first.end),
            ("អរគុណ", Origin::Completion, 0, 4)
        );
        assert_eq!(engine.suggest("", 5), []);
    }
}
