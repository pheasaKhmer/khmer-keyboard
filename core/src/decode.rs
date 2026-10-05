//! Convert romanized text to Khmer with a Viterbi search over word sequences, ported from
//! khmer-engine's `decode.py`.
//!
//! The input is split into phrases (runs of Latin words separated only by spaces); the
//! text between phrases is copied through. In a phrase, a span covers one to three typed
//! words ("or kun" for អរគុណ) or a piece of one ("soksabay|te"). The search keeps a small
//! beam of hypotheses per letter position and picks the sequence with the best total of
//! emission and bigram scores. Positions and offsets count characters, like the engine.

use std::collections::HashMap;

use crate::data::Data;
use crate::model::{bigram_logprob, logprob};
use crate::score::{descending, sort_by_score};

/// Where a reading comes from. The names match the engine's `source` strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Pronunciation,
    Spelling,
    Ungegn,
    Curated,
    English,
    Fallback,
    Typed,
    Learned,
    Completion,
}

impl Origin {
    pub fn name(self) -> &'static str {
        match self {
            Origin::Pronunciation => "pronunciation",
            Origin::Spelling => "spelling",
            Origin::Ungegn => "ungegn",
            Origin::Curated => "curated",
            Origin::English => "english",
            Origin::Fallback => "fallback",
            Origin::Typed => "typed",
            Origin::Learned => "learned",
            Origin::Completion => "completion",
        }
    }
}

/// One reading of a span of typed text. English and typed readings keep the typed text;
/// every other origin is Khmer.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub text: String,
    pub emission: f64,
    pub origin: Origin,
    pub spelling: String,
}

impl Choice {
    pub fn is_khmer(&self) -> bool {
        !matches!(self.origin, Origin::English | Origin::Typed)
    }
}

/// A span of the input and its ranked readings; `choices[0]` is the one used.
#[derive(Clone, Debug)]
pub struct Token {
    pub typed: String,
    /// Character offsets in the input, so a caller can replace the span.
    pub start: usize,
    pub end: usize,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug)]
pub struct Conversion {
    pub text: String,
    pub tokens: Vec<Token>,
    /// The n best conversions of the whole input, best first.
    pub alternatives: Vec<String>,
}

fn is_latin(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || ('\u{c0}'..='\u{24f}').contains(&ch)
        || ch == '\''
        || ch == '\u{2019}'
}

/// Python's `\s` without the newline: Unicode whitespace plus the separators U+001C-001F.
fn is_space(ch: char) -> bool {
    ch != '\n' && (ch.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&ch))
}

/// Typed words separated by spaces. Positions count letters, ignoring the spaces.
#[derive(Debug, Default)]
pub(crate) struct Phrase {
    /// Each word as characters, with its start and end offset in the input.
    words: Vec<(Vec<char>, usize, usize)>,
}

impl Phrase {
    fn offsets(&self) -> Vec<usize> {
        let mut out = vec![0];
        for (word, _, _) in &self.words {
            out.push(out[out.len() - 1] + word.len());
        }
        out
    }

    fn typed(&self, start: usize, end: usize) -> String {
        let mut pieces: Vec<String> = Vec::new();
        for ((word, _, _), offset) in self.words.iter().zip(self.offsets()) {
            let from = start.saturating_sub(offset).min(word.len());
            let to = end.saturating_sub(offset).min(word.len());
            if from < to {
                pieces.push(word[from..to].iter().collect());
            }
        }
        pieces.join(" ")
    }

    fn characters(&self, start: usize, end: usize) -> (usize, usize) {
        let offsets = self.offsets();
        let inner = &offsets[..offsets.len() - 1];
        let first = inner.iter().rposition(|&o| o <= start).unwrap_or(0);
        let last = inner.iter().rposition(|&o| o < end).unwrap_or(0);
        (
            self.words[first].1 + start - offsets[first],
            self.words[last].1 + end - offsets[last],
        )
    }

    fn joined(&self) -> String {
        self.words
            .iter()
            .map(|(w, _, _)| w.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(crate) enum Segment {
    Text(String),
    Phrase(Phrase),
}

/// Split input into phrases and the text between them.
pub(crate) fn segments(text: &str) -> Vec<Segment> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut phrase = Phrase::default();
    let mut pending_space = String::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        if is_latin(chars[i]) {
            while i < chars.len() && is_latin(chars[i]) {
                i += 1;
            }
            phrase.words.push((chars[start..i].to_vec(), start, i));
            pending_space.clear();
            continue;
        }
        let is_run = is_space(chars[i]);
        if is_run {
            while i < chars.len() && is_space(chars[i]) {
                i += 1;
            }
        } else {
            i += 1;
        }
        let piece: String = chars[start..i].iter().collect();
        if is_run && !phrase.words.is_empty() {
            pending_space = piece;
            continue;
        }
        if !phrase.words.is_empty() {
            out.push(Segment::Phrase(std::mem::take(&mut phrase)));
        }
        if !pending_space.is_empty() {
            out.push(Segment::Text(std::mem::take(&mut pending_space)));
        }
        out.push(Segment::Text(piece));
    }
    if !phrase.words.is_empty() {
        out.push(Segment::Phrase(phrase));
    }
    if !pending_space.is_empty() {
        out.push(Segment::Text(pending_space));
    }
    out
}

/// Khmer words are written together; English and typed words get a space on each side.
pub fn join(choices: &[&Choice]) -> String {
    let mut out = String::new();
    for (i, choice) in choices.iter().enumerate() {
        if i > 0 && (!choice.is_khmer() || !choices[i - 1].is_khmer()) {
            out.push(' ');
        }
        out.push_str(&choice.text);
    }
    out
}

struct Hypothesis {
    score: f64,
    previous: Option<String>,
    back: Option<usize>,
    span: (usize, usize),
    choice: Option<Choice>,
}

struct Span {
    range: (usize, usize),
    choices: Vec<Choice>,
    cost: f64,
}

/// Searches with readings from `choices(typed, whole)`: the typed text of a span (several
/// typed words keep their spaces), and whether it is made of whole typed words. Pieces of
/// a typed word (`whole` false) should only match exactly. [`crate::Engine`] supplies the
/// readings from the data; tests and other tools can supply their own.
pub struct Decoder<'a, F: FnMut(&str, bool) -> Vec<Choice>> {
    pub data: &'a Data,
    pub choices: F,
}

impl<F: FnMut(&str, bool) -> Vec<Choice>> Decoder<'_, F> {
    /// Weighted log probability of `choice` after the Khmer word `previous`.
    pub fn language_model(&self, previous: Option<&str>, choice: &Choice) -> f64 {
        let settings = &self.data.settings;
        if !choice.is_khmer() {
            return -settings.english;
        }
        let score = match previous {
            None => logprob(self.data, &choice.text),
            Some(previous) => bigram_logprob(self.data, previous, &choice.text),
        };
        settings.language_model * score
    }

    fn ranked(&mut self, typed: &str, whole: bool) -> Vec<Choice> {
        let choices = (self.choices)(typed, whole);
        let mut ranked = sort_by_score(choices, |c| c.emission + self.language_model(None, c));
        ranked.truncate(self.data.settings.choices_per_span);
        ranked
    }

    fn spans(&mut self, phrase: &Phrase) -> Vec<Span> {
        let settings = self.data.settings.clone();
        let offsets = phrase.offsets();
        let n = phrase.words.len();
        let mut out = Vec::new();
        for i in 0..n {
            for j in i + 1..=(i + settings.max_words_per_span).min(n) {
                let typed = phrase.words[i..j]
                    .iter()
                    .map(|(w, _, _)| w.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ");
                let choices = self.ranked(&typed, true);
                let cost = settings.join * (j - i - 1) as f64;
                out.push(Span {
                    range: (offsets[i], offsets[j]),
                    choices,
                    cost,
                });
            }
        }
        for (i, (word, _, _)) in phrase.words.iter().enumerate() {
            let length = word.len();
            if length < settings.min_split_length {
                continue;
            }
            for a in 0..length {
                for b in a + 2..=(a + settings.max_piece_length).min(length) {
                    if a == 0 && b == length {
                        continue; // the whole word is above
                    }
                    // Charge each split once, on the piece that ends inside the word.
                    let cost = if b < length { settings.split } else { 0.0 };
                    let piece: String = word[a..b].iter().collect();
                    let choices = self.ranked(&piece, false);
                    out.push(Span {
                        range: (offsets[i] + a, offsets[i] + b),
                        choices,
                        cost,
                    });
                }
            }
        }
        out.retain(|span| !span.choices.is_empty());
        out
    }

    fn add(&self, arena: &[Hypothesis], beam: &mut Vec<usize>, new: usize) {
        let by_score = |a: &usize, b: &usize| descending(arena[*a].score, arena[*b].score);
        for i in 0..beam.len() {
            if arena[beam[i]].previous == arena[new].previous {
                // Same context from here on: keep the better one.
                if arena[new].score > arena[beam[i]].score {
                    beam[i] = new;
                    beam.sort_by(by_score);
                }
                return;
            }
        }
        beam.push(new);
        beam.sort_by(by_score);
        beam.truncate(self.data.settings.beam);
    }

    fn path(arena: &[Hypothesis], last: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut node = Some(last);
        while let Some(i) = node {
            if arena[i].choice.is_none() {
                break;
            }
            out.push(i);
            node = arena[i].back;
        }
        out.reverse();
        out
    }

    /// Convert `text`, keeping the n best readings of each span and of the whole.
    pub fn convert(&mut self, text: &str, n: usize) -> Conversion {
        let mut pieces: Vec<Vec<String>> = Vec::new();
        let mut tokens = Vec::new();
        for segment in segments(text) {
            let phrase = match segment {
                Segment::Text(text) => {
                    pieces.push(vec![text]);
                    continue;
                }
                Segment::Phrase(phrase) => phrase,
            };
            let spans = self.spans(&phrase);
            let (arena, finals) = self.search(&phrase, &spans);
            if finals.is_empty() {
                pieces.push(vec![phrase.joined()]);
                continue;
            }
            let mut readings: Vec<String> = Vec::new();
            for &last in &finals {
                let path = Self::path(&arena, last);
                let choices: Vec<&Choice> = path
                    .iter()
                    .filter_map(|&i| arena[i].choice.as_ref())
                    .collect();
                let reading = join(&choices);
                if !readings.contains(&reading) {
                    readings.push(reading);
                }
            }
            readings.truncate(n);
            pieces.push(readings);
            tokens.extend(self.tokens(&phrase, &arena, finals[0], &spans, n));
        }
        pieces.retain(|p| !p.is_empty());
        let best: String = pieces.iter().map(|p| p[0].as_str()).collect();
        // Alternatives change one phrase at a time, keeping the best reading elsewhere.
        let mut alternatives = vec![best.clone()];
        for (i, options) in pieces.iter().enumerate() {
            for option in &options[1..] {
                let reading: String = pieces
                    .iter()
                    .enumerate()
                    .map(|(j, p)| {
                        if i == j {
                            option.as_str()
                        } else {
                            p[0].as_str()
                        }
                    })
                    .collect();
                alternatives.push(reading);
            }
        }
        alternatives.truncate(n);
        Conversion {
            text: best,
            tokens,
            alternatives,
        }
    }

    fn search(&self, phrase: &Phrase, spans: &[Span]) -> (Vec<Hypothesis>, Vec<usize>) {
        let mut starting: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, span) in spans.iter().enumerate() {
            starting.entry(span.range.0).or_default().push(i);
        }
        let length = phrase.offsets()[phrase.words.len()];
        let mut arena = vec![Hypothesis {
            score: 0.0,
            previous: None,
            back: None,
            span: (0, 0),
            choice: None,
        }];
        let mut beams: HashMap<usize, Vec<usize>> = HashMap::from([(0, vec![0])]);
        for position in 0..length {
            let Some(here) = starting.get(&position) else {
                continue;
            };
            for &s in here {
                let span = &spans[s];
                let end = span.range.1;
                let current = beams.get(&position).cloned().unwrap_or_default();
                for h in current {
                    for choice in &span.choices {
                        let mut score = arena[h].score + choice.emission - span.cost;
                        score += self.language_model(arena[h].previous.as_deref(), choice);
                        let previous = choice.is_khmer().then(|| choice.text.clone());
                        arena.push(Hypothesis {
                            score,
                            previous,
                            back: Some(h),
                            span: (position, end),
                            choice: Some(choice.clone()),
                        });
                        let new = arena.len() - 1;
                        let mut beam = beams.remove(&end).unwrap_or_default();
                        self.add(&arena, &mut beam, new);
                        beams.insert(end, beam);
                    }
                }
            }
        }
        let finals = beams.remove(&length).unwrap_or_default();
        (arena, finals)
    }

    fn tokens(
        &self,
        phrase: &Phrase,
        arena: &[Hypothesis],
        last: usize,
        spans: &[Span],
        n: usize,
    ) -> Vec<Token> {
        let by_range: HashMap<(usize, usize), &Span> = spans.iter().map(|s| (s.range, s)).collect();
        let mut tokens = Vec::new();
        let mut previous: Option<String> = None;
        for step in Self::path(arena, last) {
            let hypothesis = &arena[step];
            let chosen = hypothesis
                .choice
                .clone()
                .expect("steps on a path have choices");
            let (start, end) = hypothesis.span;
            let others: Vec<Choice> = by_range[&(start, end)]
                .choices
                .iter()
                .filter(|c| **c != chosen)
                .cloned()
                .collect();
            let others = sort_by_score(others, |c| {
                c.emission + self.language_model(previous.as_deref(), c)
            });
            let mut choices = vec![chosen];
            choices.extend(others);
            choices.truncate(n);
            let (first, end_char) = phrase.characters(start, end);
            tokens.push(Token {
                typed: phrase.typed(start, end),
                start: first,
                end: end_char,
                choices,
            });
            previous.clone_from(&hypothesis.previous);
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::{Choice, Origin, Segment, join, segments};

    #[test]
    fn segments_keep_text_between_phrases() {
        let pieces = segments("  Kâmpŭchéa l'or? ok\n12");
        let shown: Vec<String> = pieces
            .iter()
            .map(|s| match s {
                Segment::Text(t) => t.clone(),
                Segment::Phrase(p) => format!("[{}]", p.joined()),
            })
            .collect();
        assert_eq!(
            shown,
            ["  ", "[Kâmpŭchéa l'or]", "?", " ", "[ok]", "\n", "1", "2"]
        );
    }

    #[test]
    fn phrase_positions_count_letters_without_spaces() {
        let Segment::Phrase(phrase) = segments("or kunbong").remove(0) else {
            panic!("expected a phrase");
        };
        assert_eq!(phrase.offsets(), [0, 2, 9]);
        assert_eq!(phrase.typed(0, 5), "or kun");
        assert_eq!(phrase.characters(0, 5), (0, 6));
        assert_eq!(phrase.characters(5, 9), (6, 10));
    }

    #[test]
    fn khmer_is_written_together_and_english_apart() {
        let choice = |text: &str, origin| Choice {
            text: text.to_owned(),
            emission: 0.0,
            origin,
            spelling: String::new(),
        };
        let words = [
            choice("អត់", Origin::Pronunciation),
            choice("មាន", Origin::Pronunciation),
            choice("wifi", Origin::English),
            choice("ទេ", Origin::Pronunciation),
        ];
        assert_eq!(join(&words.iter().collect::<Vec<_>>()), "អត់មាន wifi ទេ");
    }

    #[test]
    fn the_next_word_decides_between_homophones() {
        use super::Decoder;
        use crate::data::sample;

        let data = sample();
        let reading = |text: &str, emission| Choice {
            text: text.to_owned(),
            emission,
            origin: Origin::Pronunciation,
            spelling: String::new(),
        };
        let choices = |typed: &str, _whole: bool| match typed {
            "bong" => vec![reading("បង", -0.5), reading("បង់", -0.5)],
            "pros" => vec![reading("ប្រុស", 0.0)],
            "luy" => vec![reading("លុយ", 0.0)],
            _ => vec![],
        };
        let mut decoder = Decoder {
            data: &data,
            choices,
        };
        // The sample has the pairs បងប្រុស (older brother) and បង់លុយ (pay money).
        assert_eq!(decoder.convert("bong pros", 5).text, "បងប្រុស");
        let result = decoder.convert("bong luy", 5);
        assert_eq!(result.text, "បង់លុយ");
        let first = &result.tokens[0];
        assert_eq!(
            (first.typed.as_str(), first.start, first.end),
            ("bong", 0, 4)
        );
        assert_eq!(first.choices[1].text, "បង");
        // Text no reading covers is kept.
        assert_eq!(decoder.convert("xyz bong", 5).text, "xyz bong");
    }
}
