//! Romanize Khmer text, ported from khmer-engine's `Romanizer.romanize`.
//!
//! Text is normalized (see [`crate::segment::normalize`]), each run of Khmer letters is
//! segmented into words, and each lexicon word is replaced by the romanization the engine
//! exported for it, in the chat style or UNGEGN. Words are separated by spaces, Khmer
//! punctuation becomes Latin punctuation, and ៗ repeats the word before it. Words the
//! lexicon does not know stay in Khmer script: spelling them needs the engine's
//! rule-based romanizers, which the core does not have yet.

use crate::data::Data;
use crate::segment::{
    LEK_TOO, Segmenter, Word, is_alnum, is_khmer_letter, merge_unknown, normalize,
};

/// The romanization style: how people type in chat, or the UNGEGN standard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Chat,
    Ungegn,
}

fn punctuation(ch: char) -> Option<&'static str> {
    Some(match ch {
        '។' | '៕' => ".",
        '៖' => ":",
        '៘' | '៚' => "...",
        '៙' => "",
        '៛' => " riel",
        _ => return None,
    })
}

/// The output, built piece by piece like the engine's list of strings. Whether a space
/// goes before a piece depends on the last piece added, which can be empty (៙ is dropped,
/// and a word can romanize to nothing), so the last character of that piece is kept.
#[derive(Default)]
struct Output {
    text: String,
    last: Option<char>,
}

impl Output {
    fn push(&mut self, piece: &str) {
        self.text.push_str(piece);
        self.last = piece.chars().next_back();
    }

    /// Add `piece`, with a space if it would otherwise run into the previous word.
    fn append(&mut self, piece: &str) {
        if let (Some(last), Some(first)) = (self.last, piece.chars().next())
            && is_alnum(last)
            && is_alnum(first)
        {
            self.text.push(' ');
        }
        self.push(piece);
    }
}

fn word(data: &Data, word: &Word, style: Style) -> String {
    match data.word_id(&word.text).filter(|_| word.known) {
        Some(id) if style == Style::Chat => data.chat(id).to_owned(),
        Some(id) => data.ungegn(id).to_owned(),
        None => word.text.clone(),
    }
}

/// The text between Khmer runs: punctuation mapped, ៗ expanded. `last_word` is the word
/// ៗ repeats; anything but a space ends it.
fn between(out: &mut Output, text: &str, last_word: &mut String) {
    for (i, ch) in text.chars().enumerate() {
        if ch == LEK_TOO && !last_word.is_empty() {
            out.append(&format!(" {}", *last_word));
        } else if let Some(mapped) = punctuation(ch) {
            out.push(mapped);
        } else {
            let mut buffer = [0; 4];
            let piece = ch.encode_utf8(&mut buffer);
            if i == 0 {
                out.append(piece); // only the first can touch a word
            } else {
                out.push(piece);
            }
            // Python's `isspace` also counts the separators U+001C-001F.
            if !ch.is_whitespace() && !('\u{1c}'..='\u{1f}').contains(&ch) {
                last_word.clear();
            }
        }
    }
}

/// Romanize Khmer text, leaving anything that is not Khmer as it is.
pub fn romanize(data: &Data, text: &str, style: Style) -> String {
    let text = normalize(text);
    let chars: Vec<char> = text.chars().collect();
    let segmenter = Segmenter::new(data);
    let mut out = Output::default();
    let mut last_word = String::new();
    let mut position = 0;
    let mut i = 0;
    while i < chars.len() {
        if !is_khmer_letter(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && is_khmer_letter(chars[i]) {
            i += 1;
        }
        let before: String = chars[position..start].iter().collect();
        between(&mut out, &before, &mut last_word);
        let run: String = chars[start..i].iter().collect();
        let words: Vec<String> = merge_unknown(segmenter.segment(&run))
            .iter()
            .map(|w| word(data, w, style))
            .collect();
        out.append(&words.join(" "));
        last_word = words.last().cloned().unwrap_or_default();
        position = i;
    }
    let rest: String = chars[position..].iter().collect();
    between(&mut out, &rest, &mut last_word);
    out.text
}

#[cfg(test)]
mod tests {
    use super::{Style, romanize};
    use crate::data::sample;

    #[test]
    fn both_styles() {
        let data = sample();
        assert_eq!(
            romanize(&data, "សួស្តី! សុខសប្បាយទេ?", Style::Chat),
            "suosdey! soksabay te?"
        );
        assert_eq!(romanize(&data, "សុខសប្បាយទេ", Style::Ungegn), "sŏkhsâbbay té");
    }

    #[test]
    fn digits_punctuation_and_repeats() {
        let data = sample();
        assert_eq!(romanize(&data, "ឆ្នាំ២០២៦", Style::Chat), "chhnam 2026");
        assert_eq!(romanize(&data, "ផ្សេងៗ។", Style::Chat), "phseng phseng.");
        assert_eq!(romanize(&data, "ខ្ញុំ love អូន", Style::Chat), "khnhom love oun");
        // ៙ is dropped, and the engine then adds no space after it.
        assert_eq!(romanize(&data, "សួស្តី៙បង", Style::Chat), "suosdeybong");
        assert_eq!(
            romanize(&data, "ផ្សេង\u{1c}ៗ", Style::Chat),
            "phseng\u{1c} phseng"
        );
    }

    #[test]
    fn coeng_da_is_read_as_coeng_ta() {
        let data = sample();
        assert_eq!(
            romanize(&data, "សួស្ដី", Style::Chat),
            romanize(&data, "សួស្តី", Style::Chat)
        );
    }

    #[test]
    fn unknown_words_stay_in_khmer() {
        let data = sample();
        assert_eq!(romanize(&data, "ហ្ឫទ័យ", Style::Chat), "ហ្ឫទ័យ");
    }
}
