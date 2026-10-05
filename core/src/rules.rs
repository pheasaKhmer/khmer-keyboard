//! Rule-based romanization of Khmer, from the spelling alone, ported from khmer-engine's
//! `rules.py`. The core uses it for words the lexicon does not know.
//!
//! Two styles:
//!
//! - UNGEGN follows the UNGEGN report on Khmer romanization (version 4.0, September 2013,
//!   <https://www.eki.ee/wgrs/rom1_km.pdf>). Table and note numbers in comments refer to it.
//! - Chat follows the Geographic Department system Cambodia has used since 1997 (the
//!   report's "Other systems of romanization", and Wikipedia's tables for the gaps): the
//!   same consonants without apostrophes, and vowels without diacritics. It is the closest
//!   standard to how people type Khmer in Latin letters.

use crate::romanize::Style;
use crate::script::{
    MUUSIKATOAN, NIKAHIT, REAHMUK, SAMYOK_SANNYA, Series, YUUKALEAPINTU, is_independent_vowel,
};
use crate::syllables::{Syllable, syllables};

/// Tables I and II. Initial and final consonants use the same letters.
fn consonant_letters(letter: char) -> &'static str {
    match letter {
        'ក' | 'គ' => "k",
        'ខ' | 'ឃ' => "kh",
        'ង' => "ng",
        'ច' | 'ជ' => "ch",
        'ឆ' | 'ឈ' => "chh",
        'ញ' => "nh",
        'ដ' | 'ឌ' => "d",
        'ឋ' | 'ឍ' | 'ថ' | 'ធ' => "th",
        'ណ' | 'ន' => "n",
        'ត' | 'ទ' => "t",
        'ប' => "b",
        'ផ' | 'ភ' => "ph",
        'ព' => "p",
        'ម' => "m",
        'យ' => "y",
        'រ' => "r",
        'ល' | 'ឡ' => "l",
        'វ' => "v",
        'ស' | 'ឝ' | 'ឞ' => "s", // ឝ and ឞ are obsolete, not in the tables
        'ហ' => "h",
        'អ' => "'",
        // Onsets and finals are always consonants (see `syllables`).
        _ => "",
    }
}

// Keys of the nucleus tables: the dependent vowel, optionally followed by nikahit or
// reahmuk, or one of these named combinations.
const INHERENT: &str = "";
const INHERENT_BANTOC: &str = "bantoc";
const AA_BANTOC: &str = "ា+bantoc";
const SAMYOK: &str = "samyok";
const SAMYOK_Y: &str = "samyok+y";
const AAM_NG: &str = "ាំង";

/// Tables IV and V, as (a-series, o-series). Where the o-series value depends on the final
/// consonant, it is "oă|eă" (note 6). ឹ is œ with a combining breve.
fn ungegn_nucleus(key: &str) -> Option<[&'static str; 2]> {
    Some(match key {
        INHERENT => ["â", "ô"],        // table I
        INHERENT_BANTOC => ["á", "ó"], // V.1
        "ា" => ["a", "éa"],
        AA_BANTOC | SAMYOK => ["ă", "oă|eă"], // V.2, V.3
        SAMYOK_Y => ["ăy", "oăy"],            // not given; V.3 before y
        "ៈ" => ["ă", "eă"],                   // not given; Wikipedia's table
        "ិ" => ["ĕ", "ĭ"],
        "ី" => ["ei", "i"],
        "ឹ" => ["œ\u{306}", "œ\u{306}"],
        "ឺ" => ["œ", "œ"],
        "ុ" => ["ŏ", "ŭ"],
        "ូ" => ["o", "u"],
        "ួ" => ["uŏ", "uŏ"],
        "ើ" => ["aeu", "eu"],
        "ឿ" => ["œă", "œă"],
        "ៀ" => ["iĕ", "iĕ"],
        "េ" => ["é", "é"],
        "ែ" => ["ê", "ê"],
        "ៃ" => ["ai", "ey"],
        "ោ" => ["aô", "oŭ"],
        "ៅ" => ["au", "ŏu"],
        "ំ" => ["âm", "um"],        // V.4
        "ុំ" => ["om", "ŭm"],        // V.5
        "ាំ" => ["ăm", "ŏâm"],      // V.6
        AAM_NG => ["ăng", "eăng"], // V.11
        "ះ" => ["ăh", "eăh"],      // V.7
        "ុះ" => ["ŏh", "ŭh"],       // V.8
        "េះ" => ["éh", "éh"],      // V.9
        "ោះ" => ["aôh", "ŏăh"],    // V.10
        _ => return None,
    })
}

/// Table III. ឧ may also be ŭ (note 10); ŏ is the default.
fn ungegn_independent(vowel: char) -> &'static str {
    match vowel {
        'ឣ' => "â",
        'ឤ' => "a",
        'ឥ' => "ĕ",
        'ឦ' => "ei",
        'ឧ' => "ŏ",
        'ឨ' => "ŏk",
        'ឩ' => "o",
        'ឪ' => "âu",
        'ឫ' => "rœ\u{306}",
        'ឬ' => "rœ",
        'ឭ' => "lœ\u{306}",
        'ឮ' => "lœ",
        'ឯ' => "ê",
        'ឰ' => "ai",
        'ឱ' | 'ឲ' => "aô",
        'ឳ' => "au",
        _ => "",
    }
}

/// Geographic Department values. Two cells differ from that system to match how people
/// type: ែ is "ae" in both series (the system has "eae" in the o-series) and ិះ is "ih"
/// in the o-series (the system has "is").
fn chat_nucleus(key: &str) -> Option<[&'static str; 2]> {
    Some(match key {
        INHERENT | INHERENT_BANTOC => ["a", "o"],
        "ា" => ["a", "ea"],
        AA_BANTOC | SAMYOK => ["a", "oa|ea"],
        SAMYOK_Y | "ៃ" => ["ai", "ey"],
        "ៈ" => ["ak", "eak"],
        "ិ" => ["e", "i"],
        "ី" => ["ei", "i"],
        "ឹ" => ["oe", "ue"],
        "ឺ" => ["eu", "ueu"],
        "ុ" => ["o", "u"],
        "ូ" => ["ou", "u"],
        "ួ" => ["uo", "uo"],
        "ើ" => ["aeu", "eu"],
        "ឿ" => ["oea", "oea"],
        "ៀ" => ["ie", "ie"],
        "េ" => ["e", "e"],
        "ែ" => ["ae", "ae"],
        "ោ" => ["ao", "ou"],
        "ៅ" => ["au", "ov"],
        "ំ" => ["am", "um"],
        "ុំ" => ["om", "um"],
        "ាំ" => ["am", "oam"],
        AAM_NG => ["ang", "eang"],
        "ះ" => ["ah", "eah"],
        "ិះ" => ["eh", "ih"],
        "ុះ" => ["oh", "uh"],
        "េះ" => ["eh", "eh"],
        "ោះ" => ["aoh", "uoh"],
        _ => return None,
    })
}

fn chat_independent(vowel: char) -> &'static str {
    match vowel {
        'ឣ' | 'ឤ' => "a",
        'ឥ' => "e",
        'ឦ' => "ei",
        'ឧ' => "o",
        'ឨ' => "ok",
        'ឩ' => "ou",
        'ឪ' | 'ឳ' => "au",
        'ឫ' => "rue",
        'ឬ' => "rueu",
        'ឭ' => "lue",
        'ឮ' => "lueu",
        'ឯ' => "ae",
        'ឰ' => "ai",
        'ឱ' | 'ឲ' => "ao",
        _ => "",
    }
}

fn nucleus(key: &str, style: Style) -> Option<[&'static str; 2]> {
    match style {
        Style::Ungegn => ungegn_nucleus(key),
        Style::Chat => chat_nucleus(key),
    }
}

fn independent(vowel: char, style: Style) -> &'static str {
    match style {
        Style::Ungegn => ungegn_independent(vowel),
        Style::Chat => chat_independent(vowel),
    }
}

/// Note 6: the o-series value is eă before these finals, otherwise oă.
const EA_FINALS: [&str; 4] = ["k", "kh", "ng", "h"];

/// The table key for a syllable's nucleus, and how many finals the key already spells.
fn nucleus_key(syllable: &Syllable) -> (String, usize) {
    let (vowel, signs, finals) = (&syllable.vowel, &syllable.signs, &syllable.finals);
    if signs.contains(SAMYOK_SANNYA) {
        if finals.first() == Some(&'យ') {
            return (SAMYOK_Y.to_owned(), 1);
        }
        return (SAMYOK.to_owned(), 0);
    }
    if signs.contains(NIKAHIT) {
        if vowel == "ា" && finals.first() == Some(&'ង') {
            return (AAM_NG.to_owned(), 1);
        }
        return (format!("{vowel}{NIKAHIT}"), 0);
    }
    if signs.contains(REAHMUK) {
        return (format!("{vowel}{REAHMUK}"), 0);
    }
    if signs.contains(YUUKALEAPINTU) {
        return (YUUKALEAPINTU.to_string(), 0);
    }
    if syllable.bantoc() && !finals.is_empty() {
        if vowel == "ា" {
            return (AA_BANTOC.to_owned(), 0);
        }
        if vowel.is_empty() {
            return (INHERENT_BANTOC.to_owned(), 0);
        }
    }
    (vowel.clone(), 0)
}

/// Romanize the nucleus. Returns the text and how many finals it already spells.
fn vowel(syllable: &Syllable, final_letters: &[&str], style: Style) -> (String, usize) {
    let column = usize::from(syllable.series() == Series::O);
    let (key, spelled) = nucleus_key(syllable);
    let mut value = if let Some(values) = nucleus(&key, style) {
        values[column].to_owned()
    } else {
        // Combinations the tables leave out (ិះ, ើះ, ...): the vowel, then m or h.
        let mut value: String = syllable
            .vowel
            .chars()
            .filter_map(|v| nucleus(v.encode_utf8(&mut [0; 4]), style))
            .map(|values| values[column])
            .collect();
        if syllable.signs.contains(NIKAHIT) {
            value.push('m');
        }
        if syllable.signs.contains(REAHMUK) {
            value.push('h');
        }
        value
    };
    if let Some((before_others, before_k_ng_h)) = value.split_once('|') {
        let k_ng_h = final_letters.first().is_some_and(|f| EA_FINALS.contains(f));
        value = if k_ng_h { before_k_ng_h } else { before_others }.to_owned();
    }
    (value, spelled)
}

/// Note 3: a subscript ត usually stands for ដ (d), except in ន្ត and before ្រ.
fn subscript_ta(onset: &[char], index: usize, previous: Option<&Syllable>) -> &'static str {
    let before_ro = onset.get(index + 1) == Some(&'រ');
    let after_no = index == 0 && previous.is_some_and(|p| p.finals.last() == Some(&'ន'));
    if before_ro || after_no { "t" } else { "d" }
}

fn consonant(letter: char, style: Style) -> &'static str {
    if letter == 'អ' && style == Style::Chat {
        return "";
    }
    consonant_letters(letter)
}

fn onset(syllable: &Syllable, previous: Option<&Syllable>, style: Style) -> String {
    let mut out = String::new();
    for (i, &letter) in syllable.onset.iter().enumerate() {
        let written_below = i > 0 || syllable.subscript_onset;
        if letter == 'ប'
            && !written_below
            && (syllable.onset.len() > 1 || syllable.shifter == Some(MUUSIKATOAN))
        {
            out.push('p'); // note 4
        } else if letter == 'ត' && written_below {
            out.push_str(subscript_ta(&syllable.onset, i, previous));
        } else if letter == 'អ'
            && previous.is_none()
            && syllable.onset == ['អ']
            && !syllable.vowel.is_empty()
        {
            // note 5: word-initial ' before a vowel is omitted
        } else if is_independent_vowel(letter) {
            out.push_str(independent(letter, style)); // written as a subscript: ហ្ឫទ័យ
        } else {
            out.push_str(consonant(letter, style));
        }
    }
    out
}

fn syllable_text(syllable: &Syllable, previous: Option<&Syllable>, style: Style) -> String {
    if style == Style::Chat && syllable.silent {
        return String::new(); // toandakhiat: written, not pronounced
    }
    let silent_finals = style == Style::Chat && syllable.silent_finals;
    let finals: Vec<&str> = if silent_finals {
        Vec::new()
    } else {
        syllable
            .finals
            .iter()
            .map(|&f| consonant(f, style))
            .collect()
    };
    let (head, spelled) = if let Some(letter) = syllable.independent {
        (independent(letter, style).to_owned(), 0)
    } else {
        let (mut sound, spelled) = vowel(syllable, &finals, style);
        if syllable.vowel.is_empty() && syllable.onset.iter().any(|&l| is_independent_vowel(l)) {
            sound.clear(); // the subscript independent vowel is the nucleus
        }
        (onset(syllable, previous, style) + &sound, spelled)
    };
    let robat = if syllable.robat { "r" } else { "" }; // note 7
    let rest: String = finals.iter().skip(spelled).copied().collect();
    head + robat + &rest
}

/// Romanize a word given as syllables.
pub fn romanize_syllables(parts: &[Syllable], style: Style) -> String {
    let mut out = String::new();
    let mut previous = None;
    for syllable in parts {
        out.push_str(&syllable_text(syllable, previous, style));
        previous = Some(syllable);
    }
    out
}

/// Romanize one Khmer word from its spelling.
pub fn romanize_word(word: &str, style: Style) -> String {
    romanize_syllables(&syllables(word), style)
}

#[cfg(test)]
mod tests {
    use super::romanize_word;
    use crate::romanize::Style;

    fn check(cases: &[(&str, &str)], style: Style) {
        for &(khmer, expected) in cases {
            assert_eq!(romanize_word(khmer, style), expected, "{khmer}");
        }
    }

    /// Official UNGEGN names of Cambodian provinces, written as one word. Names whose
    /// official form does not follow the report's own rules (Bântéay Méanchey, Preăh
    /// Vihéar, Rôtânôkiri, Môndól Kiri, Krŏng Preăh Sihanouk) are left out.
    #[test]
    fn province_names() {
        let provinces = [
            ("បន្ទាយ", "bântéay"),
            ("បាត់ដំបង", "bătdâmbâng"),
            ("កំពង់ចាម", "kâmpóngcham"),
            ("កំពង់ឆ្នាំង", "kâmpóngchhnăng"),
            ("កំពង់ស្ពឺ", "kâmpóngspœ"),
            ("កំពង់ធំ", "kâmpóngthum"),
            ("កំពត", "kâmpôt"),
            ("កណ្ដាល", "kândal"),
            ("កោះកុង", "kaôhkŏng"),
            ("ក្រចេះ", "krâchéh"),
            ("ភ្នំពេញ", "phnumpénh"),
            ("ព្រៃវែង", "preyvêng"),
            ("ពោធិ៍សាត់", "poŭthĭsăt"),
            ("សៀមរាប", "siĕmréab"),
            ("ស្ទឹងត្រែង", "stœ\u{306}ngtrêng"),
            ("ស្វាយរៀង", "svayriĕng"),
            ("តាកែវ", "takêv"),
            ("កែប", "kêb"),
            ("ប៉ៃលិន", "pailĭn"),
            ("ត្បូងឃ្មុំ", "tbongkhmŭm"),
            ("ឧត្តរ", "ŏtdâr"),
            ("ព្រះ", "preăh"),
        ];
        check(&provinces, Style::Ungegn);
    }

    /// Examples from the "Romanization of Khmer" article on English Wikipedia.
    #[test]
    fn wikipedia_examples() {
        let ungegn = [
            ("អក្សរខ្មែរ", "'âksârkhmêr"),
            ("កម្ពុជា", "kâmpŭchéa"),
            ("មណ្ឌល", "môndôl"),
            ("ពន្លឺ", "pônlœ"),
            ("សន្តិភាព", "sântĕphéap"),
            ("ជំនឿ", "chumnœă"),
            ("ទៅ", "tŏu"),
        ];
        check(&ungegn, Style::Ungegn);
        let chat = [
            ("អក្សរខ្មែរ", "aksarkhmaer"),
            ("កម្ពុជា", "kampuchea"),
            ("មណ្ឌល", "mondol"),
            ("ពន្លឺ", "ponlueu"),
            ("សន្តិភាព", "santepheap"),
            ("ជំនឿ", "chumnoea"),
            ("ទៅ", "tov"),
        ];
        check(&chat, Style::Chat);
    }

    /// Worked examples from the notes of the UNGEGN report (note number in the comment).
    #[test]
    fn report_examples() {
        let notes = [
            ("កក", "kâk"),         // 1
            ("អង្គ", "'ângk"),       // 1
            ("ហ៊ាង", "héang"),      // 2
            ("ញ៉ង", "nhâng"),       // 2
            ("ខ្ពង", "khpông"),      // 3
            ("ល្អ", "l'â"),          // 3
            ("ស្វាយ", "svay"),       // 3
            ("ក្ដី", "kdei"),         // 3
            ("កន្ត្រាប់", "kântrăb"),  // 3
            ("ប៉ង", "pâng"),        // 4
            ("ប៉ាតៅ", "patau"),     // 4
            ("ប្លែង", "plêng"),      // 4
            ("ប្រាប់", "prăb"),      // 4
            ("ក្អែក", "k'êk"),       // 5
            ("ចង្អៀត", "châng'iĕt"), // 5
            ("រអិល", "rô'ĕl"),      // 5
            ("អ្វី", "'vei"),         // 5
            ("អាង", "ang"),        // 5
            ("បត់", "bát"),         // 6
            ("ខ្ពស់", "khpós"),       // 6
            ("ចាក់", "chăk"),       // 6
            ("ច័ក", "chăk"),        // 6
            ("រពាក់", "rôpeăk"),    // 6
            ("មាត់", "moăt"),       // 6
            ("វ័ង្គ", "veăngk"),      // 6
            ("ភ័ព្វ", "phoăpv"),      // 6
            ("ធម៌", "thôrm"),       // 7
            ("បុណ្យ", "bŏny"),       // 9
            ("ពោធិ៍", "poŭthĭ"),     // 9
            ("ភូមិ", "phumĭ"),       // 9
        ];
        check(&notes, Style::Ungegn);
    }

    #[test]
    fn rules_are_applied_even_where_official_names_differ() {
        // The official name is Méanchey; the rules give choăy for ជ័យ (note 6).
        assert_eq!(romanize_word("មានជ័យ", Style::Ungegn), "méanchoăy");
    }

    #[test]
    fn other_words() {
        let words = [
            ("ឯក", "êk"), // independent vowel with a final
            ("ឪពុក", "âupŭk"),
            ("ធ្វើ", "thveu"),
            ("ពិះ", "pĭh"), // ិះ is not in the tables: the vowel, then h
        ];
        check(&words, Style::Ungegn);
    }

    #[test]
    fn empty_word() {
        assert_eq!(romanize_word("", Style::Ungegn), "");
        assert_eq!(romanize_word("", Style::Chat), "");
    }

    /// Geographic Department names of Cambodian provinces, as one word. Preăh Vihéar,
    /// Rotanak Kiri and Krong Preah Sihanouk do not follow the system's rules and are left
    /// out.
    #[test]
    fn chat_province_names() {
        let provinces = [
            ("បន្ទាយមានជ័យ", "banteaymeanchey"),
            ("កំពង់ចាម", "kampongcham"),
            ("កំពង់ឆ្នាំង", "kampongchhnang"),
            ("កំពង់ស្ពឺ", "kampongspueu"),
            ("កំពង់ធំ", "kampongthum"),
            ("កំពត", "kampot"),
            ("កណ្ដាល", "kandal"),
            ("កោះកុង", "kaohkong"),
            ("ក្រចេះ", "kracheh"),
            ("មណ្ឌលគិរី", "mondolkiri"),
            ("ភ្នំពេញ", "phnumpenh"),
            ("ពោធិ៍សាត់", "pousat"),
            ("សៀមរាប", "siemreab"),
            ("ស្ទឹងត្រែង", "stuengtraeng"),
            ("ស្វាយរៀង", "svayrieng"),
            ("តាកែវ", "takaev"),
            ("ឧត្តរមានជ័យ", "otdarmeanchey"),
            ("កែប", "kaeb"),
            ("ប៉ៃលិន", "pailin"),
            ("ត្បូងឃ្មុំ", "tboungkhmum"),
        ];
        check(&provinces, Style::Chat);
    }

    #[test]
    fn chat_drops_letters_marked_silent() {
        // ធិ៍ carries toandakhiat; UNGEGN keeps it (note 9), chat follows the pronunciation.
        assert_eq!(romanize_word("ពោធិ៍", Style::Ungegn), "poŭthĭ");
        assert_eq!(romanize_word("ពោធិ៍", Style::Chat), "pou");
    }

    #[test]
    fn chat_has_no_apostrophes() {
        assert_eq!(romanize_word("ចង្អៀត", Style::Chat), "changiet");
    }

    #[test]
    fn chat_uses_ae_for_ae_in_both_series() {
        // The Geographic Department writes "Prey Veaeng"; people type "veng" or "vaeng".
        assert_eq!(romanize_word("ព្រៃវែង", Style::Chat), "preyvaeng");
    }

    #[test]
    fn independent_vowel_as_subscript() {
        // An independent vowel written as a subscript is the syllable's vowel.
        let cases = [
            ("ហ្ឫទ័យ", "hrœ\u{306}toăy", "hruetey"),
            ("សុហ្ឫទ", "sŏhrœ\u{306}t", "sohruet"),
            ("អម្ឫត", "'âmrœ\u{306}t", "amruet"),
        ];
        for (khmer, ungegn, chat) in cases {
            assert_eq!(romanize_word(khmer, Style::Ungegn), ungegn, "{khmer}");
            assert_eq!(romanize_word(khmer, Style::Chat), chat, "{khmer}");
        }
    }
}
