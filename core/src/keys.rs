//! Matching keys for romanized Khmer.
//!
//! A port of khmer-engine's `keys.py`; the golden tests check that both give the same key
//! for every input. People spell one Khmer word many ways in Latin letters ("sous dey",
//! "suosdey", "sursdey"), and the key folds those variants together:
//!
//! - consonants: c/ch/j, chh, k/kh/g/gh/q, ph/p, th/t, w/v, nh/ny, x/s
//! - vowel spellings that chat uses for one sound: o/ou/u/ao, e/ae/eu/i, ea/ia/ie
//! - a final i after another vowel, which is the glide y: "sabai" is "sabay"
//! - r after a vowel, which chat uses to lengthen it: "orkun", "khmer"
//! - h or s at the end of a word, which are both pronounced h: "preah", "pros"
//! - doubled letters: "sabbay", and a vowel letter typed twice: "tgnaii" is "tgnai"
//! - diacritics and apostrophes: "Kâmpŭchéa", "l'â"
//!
//! Keys are lowercase consonants and uppercase vowel groups.

use unicode_normalization::UnicodeNormalization;

/// Lowercase ASCII letters only: no diacritics, apostrophes, spaces or digits.
pub fn fold(text: &str) -> String {
    let lower = text.to_lowercase().replace('œ', "oe").replace('æ', "ae");
    lower.nfd().filter(char::is_ascii_lowercase).collect()
}

/// Two-letter units, tried after "chh" and before single letters.
const DIGRAPHS: [&str; 8] = ["ch", "kh", "gh", "ng", "nh", "ny", "ph", "th"];

fn units(folded: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = folded;
    while !rest.is_empty() {
        let length = if rest.starts_with("chh") {
            3
        } else if DIGRAPHS.iter().any(|d| rest.starts_with(d)) {
            2
        } else {
            1
        };
        out.push(&rest[..length]);
        rest = &rest[length..];
    }
    out
}

fn consonant(unit: &str) -> char {
    match unit {
        "c" | "ch" | "chh" | "j" => 'c',
        "g" | "gh" | "k" | "kh" | "q" => 'k',
        "ng" => 'N',
        "nh" | "ny" => 'Y',
        "p" | "ph" => 'p',
        "s" | "x" => 's',
        "t" | "th" => 't',
        "v" | "w" => 'v',
        // b d f h l m n r y z stand for themselves.
        other => other.chars().next().unwrap_or(' '),
    }
}

/// The vowel group of a run of vowel letters, if the run is one of the listed spellings.
fn vowel_group(run: &str) -> Option<char> {
    Some(match run {
        "a" => 'A',
        "o" | "ou" | "u" | "uo" | "ua" | "uoa" | "ao" | "au" => 'O',
        "e" | "ae" | "eae" | "i" | "eu" | "oe" | "ue" | "oeu" | "ueu" | "aeu" | "oea" => 'E',
        "ea" | "ia" | "ie" | "iea" | "eia" => 'J',
        "oa" => 'Q',
        _ => return None,
    })
}

fn is_vowel_letter(unit: &str) -> bool {
    matches!(unit, "a" | "e" | "i" | "o" | "u")
}

fn is_vowel_key(symbol: char) -> bool {
    matches!(symbol, 'A' | 'O' | 'E' | 'J' | 'Q')
}

fn vowel_run(run: &str, out: &mut Vec<char>) {
    // A letter typed twice counts once: "aii" is "ai".
    let mut collapsed = String::with_capacity(run.len());
    for ch in run.chars() {
        if !collapsed.ends_with(ch) {
            collapsed.push(ch);
        }
    }
    let run = collapsed.as_str();
    let glide = run.len() > 1 && run.ends_with('i');
    let run = if glide { &run[..run.len() - 1] } else { run };
    match vowel_group(run) {
        Some(group) => out.push(group),
        None => out.extend(run.chars().map(|ch| {
            vowel_group(ch.encode_utf8(&mut [0; 4])).expect("every vowel letter has a group")
        })),
    }
    if glide {
        out.push('y');
    }
}

/// The matching key of a romanized spelling. With `final_` false the text is taken to
/// continue (a syllable inside a word, or a word still being typed), so the rules for the
/// end of a word (dropping a last r, h or s) do not apply.
pub fn key(text: &str, final_: bool) -> String {
    let folded = fold(text);
    let units = units(&folded);
    let mut symbols: Vec<char> = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        if is_vowel_letter(units[i]) {
            let start = i;
            while i < units.len() && is_vowel_letter(units[i]) {
                i += 1;
            }
            vowel_run(&units[start..i].concat(), &mut symbols);
        } else {
            symbols.push(consonant(units[i]));
            i += 1;
        }
    }
    let mut out = String::with_capacity(symbols.len());
    let mut last: Option<char> = None;
    for (i, &symbol) in symbols.iter().enumerate() {
        let after_vowel = last.is_some_and(is_vowel_key);
        let next_is_vowel = symbols.get(i + 1).is_some_and(|&s| is_vowel_key(s));
        let text_end = i + 1 == symbols.len();
        let word_end = final_ && text_end;
        if symbol == 'r' && after_vowel && !next_is_vowel && (word_end || !text_end) {
            continue; // orkun, khmer
        }
        if (symbol == 'h' || symbol == 's') && after_vowel && word_end {
            continue; // preah, pros
        }
        if last == Some(symbol) {
            continue; // sabbay
        }
        out.push(symbol);
        last = Some(symbol);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{fold, key};

    #[test]
    fn folds_to_ascii_letters() {
        assert_eq!(fold("Kâmpŭchéa"), "kampuchea");
        assert_eq!(fold("l'â 2 Stœng"), "lastoeng");
    }

    #[test]
    fn variants_share_a_key() {
        for group in [
            &["sous dey", "suosdey", "sursdey", "suos'dei"][..],
            &["chong", "jong"],
            &["orkun", "okun", "or kun"],
            &["sabbay", "sabay", "sabai", "sabaii"],
            &["tgnai", "tgnaii"],
            &["sok", "sook"],
            &["kampuchea", "Kâmpŭchéa"],
        ] {
            let first = key(group[0], true);
            for spelling in group {
                assert_eq!(key(spelling, true), first, "{spelling}");
            }
        }
    }

    #[test]
    fn text_that_continues_keeps_its_last_r_h_and_s() {
        assert_eq!(key("dar", true), "dA");
        assert_eq!(key("dar", false), "dAr");
        assert_eq!(key("preah", false), "prJh");
    }

    #[test]
    fn empty() {
        assert_eq!(key("", true), "");
        assert_eq!(key("123 ?!", true), "");
    }
}
