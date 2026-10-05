//! Split Khmer text into words, ported from khmer-engine's `segment.py`, and the light
//! normalization the core applies before segmenting.

use unicode_normalization::char::is_combining_mark;

use crate::data::Data;
use crate::model::logprob;

const COENG: char = '\u{17d2}';

/// A subset of pheasa's normalization: Khmer digits become ASCII digits, zero-width spaces
/// become spaces, and coeng da becomes coeng ta (pheasa rule 3.8), which the lexicon uses.
/// Reordering marks typed out of order is not done yet.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut after_coeng = false;
    for ch in text.chars() {
        let ch = match ch {
            '\u{17e0}'..='\u{17e9}' => char::from(b'0' + (ch as u32 - 0x17e0) as u8),
            '\u{200b}' => ' ',
            '\u{178a}' if after_coeng => '\u{178f}',
            other => other,
        };
        after_coeng = ch == COENG;
        out.push(ch);
    }
    out
}

/// Whether `ch` belongs to a Khmer word: letters, vowels, signs and joiners.
pub fn is_khmer_letter(ch: char) -> bool {
    matches!(
        ch,
        '\u{1780}'..='\u{17d3}' | '\u{17dd}' | '\u{200c}' | '\u{200d}'
    )
}

fn is_base(ch: char) -> bool {
    ('\u{1780}'..='\u{17b3}').contains(&ch)
}

/// Character offsets where a written cluster starts, plus the end of the text. A cluster
/// starts at every base letter that is not written after a coeng.
fn cluster_starts(text: &[char]) -> Vec<usize> {
    let mut starts: Vec<usize> = (0..text.len())
        .filter(|&i| is_base(text[i]) && (i == 0 || text[i - 1] != COENG))
        .collect();
    if starts.first() != Some(&0) {
        starts.insert(0, 0);
    }
    starts.push(text.len());
    starts
}

/// A segmented word, and whether the lexicon knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub known: bool,
}

/// Lowest-cost split of Khmer runs into lexicon words, never inside a written cluster.
pub struct Segmenter<'a> {
    data: &'a Data,
    unknown_cost: f64,
    max_clusters: usize,
}

impl<'a> Segmenter<'a> {
    pub fn new(data: &'a Data) -> Self {
        let unknown = 1.0 / (data.total + data.vocabulary + 1) as f64;
        Segmenter {
            data,
            unknown_cost: -unknown.ln() + 4.0,
            max_clusters: 12,
        }
    }

    fn cost(&self, piece: &str) -> Option<f64> {
        let id = self.data.word_id(piece)?;
        self.data.known(id).then(|| -logprob(self.data, piece))
    }

    /// Split one run of Khmer letters (no spaces or punctuation) into words. Neighbouring
    /// unknown clusters are merged into one unknown word.
    pub fn segment(&self, run: &str) -> Vec<Word> {
        let chars: Vec<char> = run.chars().collect();
        if chars.is_empty() {
            return Vec::new();
        }
        let bounds = cluster_starts(&chars);
        let n = bounds.len() - 1;
        let mut best = vec![f64::INFINITY; n + 1];
        best[0] = 0.0;
        let mut back = vec![(0, false); n + 1];
        for i in 0..n {
            if best[i].is_infinite() {
                continue;
            }
            let unknown = best[i] + self.unknown_cost;
            if unknown < best[i + 1] {
                (best[i + 1], back[i + 1]) = (unknown, (i, false));
            }
            for j in i + 1..=(i + self.max_clusters).min(n) {
                let piece: String = chars[bounds[i]..bounds[j]].iter().collect();
                if !self.data.has_word_prefix(&piece) {
                    break;
                }
                if let Some(cost) = self.cost(&piece)
                    && best[i] + cost < best[j]
                {
                    (best[j], back[j]) = (best[i] + cost, (i, true));
                }
            }
        }
        let mut pieces = Vec::new();
        let mut j = n;
        while j > 0 {
            let (i, known) = back[j];
            pieces.push(Word {
                text: chars[bounds[i]..bounds[j]].iter().collect(),
                known,
            });
            j = i;
        }
        pieces.reverse();
        let mut out: Vec<Word> = Vec::new();
        for piece in pieces {
            match out.last_mut() {
                Some(last) if !last.known && !piece.known => last.text.push_str(&piece.text),
                _ => out.push(piece),
            }
        }
        out
    }
}

/// A single consonant cluster with no vowel, such as ក or ក្រ: the lexicon lists letters as
/// words, but next to unknown text they are usually part of it.
fn is_letter(word: &Word) -> bool {
    let chars: Vec<char> = word.text.chars().collect();
    let vocalic = |ch: char| {
        ('\u{17b6}'..='\u{17c8}').contains(&ch)
            || matches!(ch, '\u{17ce}' | '\u{17cf}' | '\u{17d0}')
    };
    cluster_starts(&chars).len() == 2
        && ('\u{1780}'..='\u{17a2}').contains(&chars[0])
        && !chars.iter().any(|&c| vocalic(c))
        && !chars
            .windows(2)
            .any(|w| w[0] == COENG && ('\u{17a3}'..='\u{17b3}').contains(&w[1]))
}

/// Join each unknown word with the bare letters around it.
pub fn merge_unknown(words: Vec<Word>) -> Vec<Word> {
    let mut out = Vec::new();
    let mut group: Vec<Word> = Vec::new();
    let flush = |group: &mut Vec<Word>, out: &mut Vec<Word>| {
        if group.iter().any(|w| !w.known) {
            let text = group.iter().map(|w| w.text.as_str()).collect();
            out.push(Word { text, known: false });
        } else {
            out.append(group);
        }
        group.clear();
    };
    for word in words {
        if !word.known || is_letter(&word) {
            group.push(word);
        } else {
            flush(&mut group, &mut out);
            out.push(word);
        }
    }
    flush(&mut group, &mut out);
    out
}

/// Python's `str.isalnum` for one character: letters and digits, but not combining marks
/// (Khmer vowel signs count as alphabetic in Rust).
pub fn is_alnum(ch: char) -> bool {
    ch.is_alphanumeric() && !is_combining_mark(ch)
}

#[cfg(test)]
mod tests {
    use super::{Segmenter, Word, merge_unknown, normalize};
    use crate::data::sample;

    fn word(text: &str, known: bool) -> Word {
        Word {
            text: text.to_owned(),
            known,
        }
    }

    #[test]
    fn normalizes_digits_spaces_and_coeng_da() {
        assert_eq!(normalize("ឆ្នាំ២០២៦\u{200b}សួស្ដី"), "ឆ្នាំ2026 សួស្តី");
        assert_eq!(normalize("ដ"), "ដ"); // only after a coeng
    }

    #[test]
    fn segments_into_lexicon_words() {
        let data = sample();
        let words = Segmenter::new(&data).segment("ខ្ញុំស្រឡាញ់អូន");
        assert_eq!(
            words,
            [word("ខ្ញុំ", true), word("ស្រឡាញ់", true), word("អូន", true)]
        );
        assert_eq!(Segmenter::new(&data).segment(""), []);
    }

    #[test]
    fn unknown_pieces_absorb_the_bare_letters_around_them() {
        let words = vec![
            word("ច", true),
            word("ក្រ", true),
            word("ត្តិ", false),
            word("ទេ", true),
        ];
        assert_eq!(
            merge_unknown(words),
            [word("ចក្រត្តិ", false), word("ទេ", true)]
        );
        let letters = vec![word("ក", true), word("ខ", true)];
        assert_eq!(merge_unknown(letters.clone()), letters);
    }
}
