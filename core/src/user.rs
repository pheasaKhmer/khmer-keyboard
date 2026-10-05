//! What the user picked before, so it ranks higher next time. Ported from khmer-engine's
//! `UserDictionary`.
//!
//! Picks are counted per matching key of the typed text, so a choice made for "sok" also
//! counts for "sork", and per previous Khmer word, so a choice made after one word can
//! count more after that word than elsewhere: "te" picked as តេ after ចាំ need not
//! displace ទេ everywhere. They live in memory and, if a path is given, in a small local
//! file (one `key<TAB>previous<TAB>word<TAB>count` line per pick, previous empty for none).
//! Nothing is sent anywhere.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::keys::key;

/// How often `word` was picked after the Khmer word `previous` ("" for none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub previous: String,
    pub word: String,
    pub count: u32,
}

#[derive(Debug, Default)]
pub struct UserDictionary {
    path: Option<PathBuf>,
    /// Each key's picks, in the order they were first made.
    counts: HashMap<String, Vec<Pick>>,
}

impl UserDictionary {
    /// Picks kept in memory only.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Picks kept in `path`, loading what is already there. Files from before picks kept
    /// the previous word have three fields; those picks count as made without one.
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
            let fields: Vec<&str> = line.split('\t').collect();
            let (key, previous, word, count) = match fields[..] {
                [key, previous, word, count] => (key, previous, word, count),
                [key, word, count] => (key, "", word, count),
                _ => continue,
            };
            if let Ok(count) = count.parse() {
                dictionary
                    .counts
                    .entry(key.to_owned())
                    .or_default()
                    .push(Pick {
                        previous: previous.to_owned(),
                        word: word.to_owned(),
                        count,
                    });
            }
        }
        Ok(dictionary)
    }

    /// Record that `word` was picked for `typed`, after the Khmer word `previous`.
    pub fn learn(&mut self, typed: &str, word: &str, previous: Option<&str>) -> io::Result<()> {
        let typed_key = key(typed, true);
        let previous = previous.unwrap_or("");
        let bad = |s: &str| s.contains(['\t', '\n']);
        if typed_key.is_empty() || word.is_empty() || bad(word) || bad(previous) {
            return Ok(());
        }
        let picks = self.counts.entry(typed_key).or_default();
        match picks
            .iter_mut()
            .find(|p| p.previous == previous && p.word == word)
        {
            Some(pick) => pick.count += 1,
            None => picks.push(Pick {
                previous: previous.to_owned(),
                word: word.to_owned(),
                count: 1,
            }),
        }
        self.save()
    }

    /// Every pick for text with the same key as `typed`.
    pub fn picks(&self, typed: &str) -> &[Pick] {
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
            for pick in &self.counts[key] {
                let _ = writeln!(
                    text,
                    "{key}\t{}\t{}\t{}",
                    pick.previous, pick.word, pick.count
                );
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
    use super::{Pick, UserDictionary};

    fn pick(previous: &str, word: &str, count: u32) -> Pick {
        Pick {
            previous: previous.to_owned(),
            word: word.to_owned(),
            count,
        }
    }

    #[test]
    fn picks_are_counted_by_matching_key_and_previous_word() {
        let mut user = UserDictionary::in_memory();
        user.learn("sok", "សុខ", None).unwrap();
        user.learn("sork", "សុខ", None).unwrap();
        user.learn("sok", "សុខ", Some("ខ្ញុំ")).unwrap();
        assert_eq!(user.picks("sok"), [pick("", "សុខ", 2), pick("ខ្ញុំ", "សុខ", 1)]);
        assert_eq!(user.picks("bong"), []);
    }

    #[test]
    fn empty_input_is_not_learned() {
        let mut user = UserDictionary::in_memory();
        user.learn("123", "ក", None).unwrap();
        user.learn("sok", "", None).unwrap();
        assert_eq!(user.picks("123"), []);
        assert_eq!(user.picks("sok"), []);
    }

    #[test]
    fn picks_are_saved_loaded_and_cleared() {
        let directory =
            std::env::temp_dir().join(format!("khmer-core-user-{}", std::process::id()));
        let path = directory.join("nested").join("picks.tsv");
        let mut user = UserDictionary::open(&path).unwrap();
        user.learn("bong", "បង់", Some("ការ")).unwrap();
        user.learn("bong", "បង់", Some("ការ")).unwrap();
        user.learn("bong", "បង", None).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "bON\tការ\tបង់\t2\nbON\t\tបង\t1\n"
        );
        let mut again = UserDictionary::open(&path).unwrap();
        assert_eq!(
            again.picks("borng"),
            [pick("ការ", "បង់", 2), pick("", "បង", 1)]
        );
        again.clear().unwrap();
        assert_eq!(again.picks("bong"), []);
        assert!(!path.exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn picks_saved_without_previous_words_still_load() {
        let directory =
            std::env::temp_dir().join(format!("khmer-core-user-old-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("picks.tsv");
        std::fs::write(&path, "bON\tបង់\t2\n").unwrap();
        let user = UserDictionary::open(&path).unwrap();
        assert_eq!(user.picks("bong"), [pick("", "បង់", 2)]);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
