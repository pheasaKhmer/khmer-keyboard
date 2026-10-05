//! Edit distance and edit neighbourhoods, ported from khmer-engine's `fuzzy.py`.

/// Optimal string alignment distance: insertions, deletions, substitutions and swaps of
/// two neighbouring letters each cost 1.
pub fn distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() || b.is_empty() {
        return a.len().max(b.len());
    }
    let mut before: Vec<usize> = Vec::new();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let i = i + 1;
        let mut current = vec![0; b.len() + 1];
        current[0] = i;
        for (j, &cb) in b.iter().enumerate() {
            let j = j + 1;
            current[j] = (previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(previous[j - 1] + usize::from(ca != cb));
            if i > 1 && j > 1 && ca == b[j - 2] && a[i - 2] == cb {
                current[j] = current[j].min(before[j - 2] + 1);
            }
        }
        before = std::mem::replace(&mut previous, current);
    }
    previous[b.len()]
}

/// Every string one edit away from `word`, in the same order as the engine (it may
/// contain duplicates). Keys are ASCII, so this works on bytes.
pub fn neighbours(word: &str, alphabet: &[u8]) -> Vec<String> {
    let bytes = word.as_bytes();
    let mut out = Vec::with_capacity((bytes.len() + 1) * (2 * alphabet.len() + 2));
    let make = |parts: &[&[u8]]| String::from_utf8(parts.concat()).expect("keys are ASCII");
    for i in 0..=bytes.len() {
        let (head, tail) = bytes.split_at(i);
        for &letter in alphabet {
            out.push(make(&[head, &[letter], tail])); // insertion
        }
        if let Some((&first, rest)) = tail.split_first() {
            out.push(make(&[head, rest])); // deletion
            for &letter in alphabet {
                if letter != first {
                    out.push(make(&[head, &[letter], rest])); // substitution
                }
            }
            if let Some((&second, rest)) = rest.split_first() {
                out.push(make(&[head, &[second, first], rest])); // swap
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{distance, neighbours};

    #[test]
    fn distances() {
        for (a, b, expected) in [
            ("", "", 0),
            ("", "abc", 3),
            ("sok", "sork", 1),
            ("sabay", "saby", 1),
            ("bong", "bang", 1),
            ("sabay", "sbaay", 1),
            ("kitten", "sitting", 3),
        ] {
            assert_eq!(distance(a, b), expected, "{a} {b}");
            assert_eq!(distance(b, a), expected, "{b} {a}");
        }
    }

    #[test]
    fn neighbours_are_one_edit_away_in_the_engine_order() {
        let found = neighbours("so", b"ab");
        assert_eq!(
            found,
            [
                "aso", "bso", "o", "ao", "bo", "os", // at 0
                "sao", "sbo", "s", "sa", "sb", // at 1
                "soa", "sob", // at 2
            ]
        );
        assert!(found.iter().all(|other| distance("so", other) == 1));
    }
}
