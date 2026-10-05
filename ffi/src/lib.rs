//! Kotlin and Swift bindings for khmer-core, made by [UniFFI](https://mozilla.github.io/uniffi-rs/).
//!
//! An app makes one [`Keyboard`]. It loads the data file, keeps the user's picks and
//! answers every keystroke. The keyboard may call it from any thread, so the engine sits
//! behind a mutex; each call is short (well under a millisecond on a desktop).

use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use khmer_core::{Data, Engine, Style, UserDictionary};

uniffi::setup_scaffolding!();

/// Why the keyboard could not load its data or save the user's picks. The field is not
/// called `message`, which every Kotlin exception already has.
#[derive(Debug, uniffi::Error)]
pub enum KeyboardError {
    /// The data file is missing, damaged or from another version of the core.
    Data { reason: String },
    /// The file of learned picks could not be read or written.
    Storage { reason: String },
}

impl fmt::Display for KeyboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyboardError::Data { reason } => write!(f, "keyboard data: {reason}"),
            KeyboardError::Storage { reason } => write!(f, "learned words: {reason}"),
        }
    }
}

impl std::error::Error for KeyboardError {}

fn storage(error: std::io::Error) -> KeyboardError {
    KeyboardError::Storage {
        reason: error.to_string(),
    }
}

/// A candidate for the suggestion bar. Replacing characters `start..end` of the typed text
/// with `text` commits it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Suggestion {
    pub text: String,
    pub start: u32,
    pub end: u32,
    /// Where the candidate came from: "curated", "consonants", "english", "typed", ...
    pub source: String,
}

/// How to romanize Khmer: the way people type it in chat, or the UNGEGN standard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RomanizationStyle {
    Chat,
    Ungegn,
}

#[derive(uniffi::Object)]
pub struct Keyboard {
    engine: Mutex<Engine>,
}

impl Keyboard {
    fn new(data: Data, user_path: Option<String>) -> Result<Arc<Self>, KeyboardError> {
        let user = match user_path {
            Some(path) => UserDictionary::open(Path::new(&path)).map_err(storage)?,
            None => UserDictionary::in_memory(),
        };
        Ok(Arc::new(Keyboard {
            engine: Mutex::new(Engine::with_user(data, user)),
        }))
    }

    fn engine(&self) -> MutexGuard<'_, Engine> {
        // A panic while holding the lock leaves the engine usable: its only mutable
        // state is a cache and the picks, which are saved whole or not at all.
        self.engine.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn data_error(error: impl fmt::Display) -> KeyboardError {
    KeyboardError::Data {
        reason: error.to_string(),
    }
}

#[uniffi::export]
impl Keyboard {
    /// Load the compiled data file at `data_path`. Learned picks are kept in the file at
    /// `user_path`, or only in memory if it is null.
    #[uniffi::constructor]
    pub fn open(data_path: String, user_path: Option<String>) -> Result<Arc<Self>, KeyboardError> {
        Self::new(
            Data::open(Path::new(&data_path)).map_err(data_error)?,
            user_path,
        )
    }

    /// Like [`Keyboard::open`], from the data file's bytes (an Android asset, say).
    #[uniffi::constructor]
    pub fn from_bytes(
        data: Vec<u8>,
        user_path: Option<String>,
    ) -> Result<Arc<Self>, KeyboardError> {
        Self::new(Data::from_bytes(data).map_err(data_error)?, user_path)
    }

    /// Up to `count` candidates for the word being typed at the end of `typed`, best first.
    /// `context` is the text already in the field before `typed`; its last Khmer word
    /// helps choose between words that sound alike.
    pub fn suggest(&self, context: String, typed: String, count: u32) -> Vec<Suggestion> {
        self.engine()
            .suggest_in_context(&context, &typed, count as usize)
            .into_iter()
            .map(|s| Suggestion {
                text: s.text,
                start: s.start as u32,
                end: s.end as u32,
                source: s.origin.name().to_owned(),
            })
            .collect()
    }

    /// Record that the user picked `word` for `typed`, so it ranks higher next time.
    /// `context` is the text before `typed`, as for [`Keyboard::suggest`]: a pick counts
    /// most after the same Khmer word.
    pub fn learn(&self, context: String, typed: String, word: String) -> Result<(), KeyboardError> {
        self.engine()
            .learn_in_context(&context, &typed, &word)
            .map_err(storage)
    }

    /// Forget every learned pick, and delete the file that kept them.
    pub fn forget(&self) -> Result<(), KeyboardError> {
        self.engine().forget().map_err(storage)
    }

    /// The best Khmer conversion of romanized `text`.
    pub fn convert(&self, text: String) -> String {
        self.engine().convert(&text)
    }

    /// Khmer text in Latin letters.
    pub fn romanize(&self, khmer: String, style: RomanizationStyle) -> String {
        let style = match style {
            RomanizationStyle::Chat => Style::Chat,
            RomanizationStyle::Ungegn => Style::Ungegn,
        };
        self.engine().romanize(&khmer, style)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use khmer_core::data;

    use super::{Keyboard, KeyboardError, RomanizationStyle};

    fn sample() -> Vec<u8> {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/sample");
        data::compile(&directory).unwrap()
    }

    #[test]
    fn suggests_converts_and_romanizes() {
        let keyboard = Keyboard::from_bytes(sample(), None).unwrap();
        let suggestions = keyboard.suggest(String::new(), "sok sabay".into(), 3);
        // One word typed as two: the suggestion replaces both.
        assert_eq!(suggestions[0].text, "សុខសប្បាយ");
        assert_eq!((suggestions[0].start, suggestions[0].end), (0, 9));
        assert!(suggestions.len() <= 3);
        assert_eq!(keyboard.convert("ban hz hz".into()), "បានហើយៗ");
        assert_eq!(
            keyboard.romanize("សុខសប្បាយទេ".into(), RomanizationStyle::Chat),
            "soksabay te"
        );
    }

    #[test]
    fn context_and_learning_change_the_order() {
        let keyboard = Keyboard::from_bytes(sample(), None).unwrap();
        let after = |context: &str| {
            keyboard.suggest(context.into(), "bong".into(), 5)[0]
                .text
                .clone()
        };
        assert_eq!(after("ការ"), "បង់");
        keyboard
            .learn(String::new(), "bong".into(), "បង់".into())
            .unwrap();
        keyboard
            .learn(String::new(), "bong".into(), "បង់".into())
            .unwrap();
        assert_eq!(after(""), "បង់");
        keyboard.forget().unwrap();
        assert_eq!(after(""), "បង");
    }

    #[test]
    fn a_pick_counts_after_the_same_word() {
        let keyboard = Keyboard::from_bytes(sample(), None).unwrap();
        for _ in 0..3 {
            keyboard
                .learn("ខ្ញុំចាំ".into(), "te".into(), "តេ".into())
                .unwrap();
        }
        let first = |context: &str| {
            keyboard.suggest(context.into(), "te".into(), 3)[0]
                .text
                .clone()
        };
        assert_eq!(first("ខ្ញុំចាំ"), "តេ");
        assert_eq!(first("អត់មាន"), "ទេ");
    }

    #[test]
    fn bad_data_is_an_error() {
        let error = Keyboard::from_bytes(b"not data".to_vec(), None)
            .err()
            .unwrap();
        assert!(matches!(error, KeyboardError::Data { .. }));
        assert!(error.to_string().starts_with("keyboard data: "));
    }
}
