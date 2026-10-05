//! Reading the tab-separated files written by `data/export.py`.
//!
//! Lines starting with `#` are comments. Fields escape backslash, tab and newline as
//! `\\`, `\t` and `\n`.

use std::fs;
use std::io;
use std::path::Path;

/// Undo the export's escaping of backslash, tab and newline.
pub fn unescape(field: &str) -> String {
    if !field.contains('\\') {
        return field.to_owned();
    }
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// The unescaped fields of every data line of a file.
pub fn read(path: &Path) -> io::Result<Vec<Vec<String>>> {
    let text = fs::read_to_string(path)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;
    Ok(parse(&text))
}

/// The unescaped fields of every data line of `text`.
pub fn parse(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split('\t').map(unescape).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse, unescape};

    #[test]
    fn unescapes() {
        assert_eq!(unescape(r"a\tb\nc\\d"), "a\tb\nc\\d");
        assert_eq!(unescape("plain"), "plain");
    }

    #[test]
    fn skips_comments_and_keeps_empty_fields() {
        let rows = parse("# header\na\t\tb\n\nc\n");
        assert_eq!(rows, [vec!["a", "", "b"], vec![""], vec!["c"]]);
    }
}
