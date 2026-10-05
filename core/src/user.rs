//! What the user picked before, so it ranks higher next time. Ported from khmer-engine's
//! `UserDictionary`.
//!
//! Picks are counted per matching key of the typed text, so a choice made for "sok" also
//! counts for "sork". They live in memory and, if a path is given, in a small local file
//! (one `key<TAB>word<TAB>count` line per pick). Nothing is sent anywhere.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::keys::key;

#[derive(Debug, Default)]
pub struct UserDictionary {
    path: Option<PathBuf>,
    /// Each key's picked words, in the order they were first picked, with counts.
    counts: HashMap<String, Vec<(String, u32)>>,
}

impl UserDictionary {
    /// Picks kept in memory only.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Picks kept in `path`, loading what is already there.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut dictionary = UserDictionary {
            path: Some(path.to_owned()),
            counts: HashMap::new(),
        };
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(dictionary),
            Err(error) => return Err(error),
        };
        for line in text.lines() {
            let mut fields = line.split('\t');
            if let (Some(key), Some(word), Some(count), None) =
                (fields.next(), fields.next(), fields.next(), fields.next())
                && let Ok(count) = count.parse()
            {
                dictionary
                    .counts
                    .entry(key.to_owned())
                    .or_default()
                    .push((word.to_owned(), count));
            }
        }
        Ok(dictionary)
    }

    /// Record that `word` was picked for `typed`.
    pub fn learn(&mut self, typed: &str, word: &str) -> io::Result<()> {
        let typed_key = key(typed, true);
        if typed_key.is_empty() || word.is_empty() || word.contains(['\t', '\n']) {
            return Ok(());
        }
        let picks = self.counts.entry(typed_key).or_default();
        match picks.iter_mut().find(|(w, _)| w == word) {
            Some((_, count)) => *count += 1,
            None => picks.push((word.to_owned(), 1)),
        }
        self.save()
    }

    /// How often each word was picked for text with the same key as `typed`.
    pub fn picks(&self, typed: &str) -> &[(String, u32)] {
        self.counts
            .get(&key(typed, true))
            .map_or(&[], Vec::as_slice)
    }

    /// Forget every pick, and delete the file.
    pub fn clear(&mut self) -> io::Result<()> {
        self.counts.clear();
        match &self.path {
            Some(path) => match fs::remove_file(path) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            },
            None => Ok(()),
        }
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let mut keys: Vec<&String> = self.counts.keys().collect();
        keys.sort();
        let mut text = String::new();
        for key in keys {
            for (word, count) in &self.counts[key] {
                let _ = writeln!(text, "{key}\t{word}\t{count}");
            }
        }
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory)?;
        }
        // Write a temporary file and rename it, so a crash never leaves half a file.
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, text)?;
        fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::UserDictionary;

    #[test]
    fn picks_are_counted_by_matching_key() {
        let mut user = UserDictionary::in_memory();
        user.learn("sok", "សុខ").unwrap();
        user.learn("sork", "សុខ").unwrap();
        assert_eq!(user.picks("sok"), [("សុខ".to_owned(), 2)]);
        assert_eq!(user.picks("bong"), []);
    }

    #[test]
    fn empty_input_is_not_learned() {
        let mut user = UserDictionary::in_memory();
        user.learn("123", "ក").unwrap();
        user.learn("sok", "").unwrap();
        assert_eq!(user.picks("123"), []);
        assert_eq!(user.picks("sok"), []);
    }

    #[test]
    fn picks_are_saved_loaded_and_cleared() {
        let directory =
            std::env::temp_dir().join(format!("khmer-core-user-{}", std::process::id()));
        let path = directory.join("nested").join("picks.tsv");
        let mut user = UserDictionary::open(&path).unwrap();
        user.learn("bong", "បង់").unwrap();
        user.learn("bong", "បង់").unwrap();
        user.learn("bong", "បង").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "bON\tបង់\t2\nbON\tបង\t1\n"
        );
        let mut again = UserDictionary::open(&path).unwrap();
        assert_eq!(
            again.picks("borng"),
            [("បង់".to_owned(), 2), ("បង".to_owned(), 1)]
        );
        again.clear().unwrap();
        assert_eq!(again.picks("bong"), []);
        assert!(!path.exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
