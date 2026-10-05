//! Khmer character classes (Unicode block U+1780-U+17FF), ported from khmer-engine's
//! `script.py`.
//!
//! Series assignments follow the UNGEGN report on Khmer romanization (version 4.0, 2013),
//! table I: consonants romanized with "â" are a-series, those with "ô" are o-series.

pub const COENG: char = '\u{17d2}';
pub const NIKAHIT: char = '\u{17c6}'; // ំ
pub const REAHMUK: char = '\u{17c7}'; // ះ
pub const YUUKALEAPINTU: char = '\u{17c8}'; // ៈ
/// ៉ moves an o-series consonant to the a-series.
pub const MUUSIKATOAN: char = '\u{17c9}';
/// ៊ moves an a-series consonant to the o-series.
pub const TRIISAP: char = '\u{17ca}';
/// ់ shortens the vowel before a final consonant.
pub const BANTOC: char = '\u{17cb}';
pub const ROBAT: char = '\u{17cc}'; // ៌
/// ៍ marks letters that are written but not pronounced.
pub const TOANDAKHIAT: char = '\u{17cd}';
pub const KAKABAT: char = '\u{17ce}'; // ៎
pub const AHSDA: char = '\u{17cf}'; // ៏
pub const SAMYOK_SANNYA: char = '\u{17d0}'; // ័
pub const VIRIAM: char = '\u{17d1}'; // ៑

/// The two consonant series, which give a vowel sign its a-series or o-series sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Series {
    A,
    O,
}

const A_SERIES: &str = "កខចឆដឋណតថបផឝសហឡអ";
const O_SERIES: &str = "គឃងជឈញឌឍទធនពភមយរលវឞ";

/// A subscript decides the series of the syllable unless it is one of these; then the
/// base consonant decides (UNGEGN note 3).
const SERIES_NEUTRAL_SUBSCRIPTS: &str = "ងញណនមយរលវស";

/// The 35 consonants, U+1780-U+17A2.
pub fn is_consonant(ch: char) -> bool {
    ('\u{1780}'..='\u{17a2}').contains(&ch)
}

/// Independent vowels, U+17A3-U+17B3, including the two deprecated ones.
pub fn is_independent_vowel(ch: char) -> bool {
    ('\u{17a3}'..='\u{17b3}').contains(&ch)
}

/// Dependent vowel signs, U+17B6-U+17C5.
pub fn is_dependent_vowel(ch: char) -> bool {
    ('\u{17b6}'..='\u{17c5}').contains(&ch)
}

pub fn is_shifter(ch: char) -> bool {
    matches!(ch, MUUSIKATOAN | TRIISAP)
}

/// Signs that carry or change the vowel of a syllable, so a cluster that has one is never
/// a bare final consonant.
pub fn is_vocalic_sign(ch: char) -> bool {
    matches!(
        ch,
        NIKAHIT | REAHMUK | YUUKALEAPINTU | SAMYOK_SANNYA | AHSDA | KAKABAT
    )
}

/// Every sign other than the shifters and coeng.
pub fn is_sign(ch: char) -> bool {
    is_vocalic_sign(ch)
        || matches!(
            ch,
            BANTOC | ROBAT | TOANDAKHIAT | VIRIAM | '\u{17d3}' | '\u{17dd}'
        )
}

/// The series of a consonant letter; `None` for anything else.
pub fn series(consonant: char) -> Option<Series> {
    if A_SERIES.contains(consonant) {
        Some(Series::A)
    } else if O_SERIES.contains(consonant) {
        Some(Series::O)
    } else {
        None
    }
}

/// Whether a subscript leaves the series to the base consonant (UNGEGN note 3).
pub fn is_series_neutral(subscript: char) -> bool {
    SERIES_NEUTRAL_SUBSCRIPTS.contains(subscript)
}

#[cfg(test)]
mod tests {
    use super::{Series, is_consonant, is_series_neutral, series};

    #[test]
    fn every_consonant_has_a_series() {
        let consonants = ('\u{1780}'..='\u{17a2}').filter(|&ch| is_consonant(ch));
        assert_eq!(consonants.clone().count(), 35);
        assert!(consonants.clone().all(|ch| series(ch).is_some()));
        assert_eq!(series('ក'), Some(Series::A));
        assert_eq!(series('គ'), Some(Series::O));
        assert_eq!(series('ឥ'), None);
    }

    #[test]
    fn sonorants_are_series_neutral() {
        assert!(is_series_neutral('វ'));
        assert!(!is_series_neutral('ព'));
    }
}
