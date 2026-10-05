//! The compiled data file: everything the core looks up, in one read-only buffer.
//!
//! [`compile`] turns the files that `data/export.py` writes into one byte buffer, and
//! [`Data::from_bytes`] reads it without copying. The buffer starts with a table of
//! sections; lookup tables (key to forms, word to id, syllables, English words) are
//! [`fst`] maps, which keep sorted keys compact and find prefixes fast, and everything else
//! is plain arrays of little-endian numbers.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use fst::{IntoStreamer, Map, MapBuilder, Set, SetBuilder, Streamer};

use crate::tsv;

const MAGIC: &[u8; 4] = b"KHKB";
const VERSION: u32 = 1;

/// Where a romanization in the index comes from (khmer-engine's `Form.source`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Pronunciation,
    Spelling,
    Ungegn,
    Curated,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Pronunciation => "pronunciation",
            Source::Spelling => "spelling",
            Source::Ungegn => "ungegn",
            Source::Curated => "curated",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "pronunciation" => Source::Pronunciation,
            "spelling" => Source::Spelling,
            "ungegn" => Source::Ungegn,
            "curated" => Source::Curated,
            _ => return None,
        })
    }

    fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Source::Pronunciation,
            1 => Source::Spelling,
            2 => Source::Ungegn,
            _ => Source::Curated,
        }
    }
}

/// A romanization of a word, under one matching key.
#[derive(Clone, Copy, Debug)]
pub struct Form<'a> {
    pub word: u32,
    pub spelling: &'a str,
    pub source: Source,
}

/// Every number the scores depend on, as tuned in khmer-engine and exported by name.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub key_edit: f64,
    pub spelling: f64,
    pub curated: f64,
    pub frequency: f64,
    pub completion: f64,
    pub missing: f64,
    pub language_model: f64,
    pub english: f64,
    pub join: f64,
    pub split: f64,
    pub max_words_per_span: usize,
    pub min_split_length: usize,
    pub max_piece_length: usize,
    pub choices_per_span: usize,
    pub beam: usize,
    pub min_fuzzy_key: usize,
    pub min_completion_key: usize,
    pub max_completion_scan: usize,
    pub fallback_emission: f64,
    pub typed_emission: f64,
    pub learned_weight: f64,
    pub learned_unknown_bonus: f64,
    pub max_syllable_letters: usize,
    pub vowel_onset_cost: f64,
}

impl Settings {
    fn from_meta(meta: &HashMap<String, String>) -> Result<Self, Error> {
        let float = |name: &str| -> Result<f64, Error> {
            let value = meta
                .get(name)
                .ok_or_else(|| Error::new(format!("meta: no {name}")))?;
            value
                .parse()
                .map_err(|_| Error::new(format!("meta: {name} is not a number")))
        };
        let count = |name: &str| -> Result<usize, Error> {
            let value = meta
                .get(name)
                .ok_or_else(|| Error::new(format!("meta: no {name}")))?;
            value
                .parse()
                .map_err(|_| Error::new(format!("meta: {name} is not a count")))
        };
        Ok(Settings {
            key_edit: float("weights.key_edit")?,
            spelling: float("weights.spelling")?,
            curated: float("weights.curated")?,
            frequency: float("weights.frequency")?,
            completion: float("weights.completion")?,
            missing: float("weights.missing")?,
            language_model: float("settings.language_model")?,
            english: float("settings.english")?,
            join: float("settings.join")?,
            split: float("settings.split")?,
            max_words_per_span: count("settings.max_words_per_span")?,
            min_split_length: count("settings.min_split_length")?,
            max_piece_length: count("settings.max_piece_length")?,
            choices_per_span: count("settings.choices_per_span")?,
            beam: count("settings.beam")?,
            min_fuzzy_key: count("match.MIN_FUZZY_KEY")?,
            min_completion_key: count("match.MIN_COMPLETION_KEY")?,
            max_completion_scan: count("match.MAX_COMPLETION_SCAN")?,
            fallback_emission: float("engine.FALLBACK_EMISSION")?,
            typed_emission: float("engine.TYPED_EMISSION")?,
            learned_weight: float("engine.LEARNED_WEIGHT")?,
            learned_unknown_bonus: float("engine.LEARNED_UNKNOWN_BONUS")?,
            max_syllable_letters: count("transliterate.MAX_SYLLABLE_LETTERS")?,
            vowel_onset_cost: float("transliterate.VOWEL_ONSET_COST")?,
        })
    }
}

/// A problem with the data files or the compiled data.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Error(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error(error.to_string())
    }
}

impl From<fst::Error> for Error {
    fn from(error: fst::Error) -> Self {
        Error(error.to_string())
    }
}

// Section tags. Each string table is a blob plus u32 offsets (one more than strings).
const META: &[u8; 4] = b"META";
const WORD_BLOB: &[u8; 4] = b"WRDB";
const WORD_OFFSETS: &[u8; 4] = b"WRDO";
const WORD_COUNTS: &[u8; 4] = b"WCNT";
const WORD_KNOWN: &[u8; 4] = b"WKNO";
const CHAT_BLOB: &[u8; 4] = b"CHTB";
const CHAT_OFFSETS: &[u8; 4] = b"CHTO";
const UNGEGN_BLOB: &[u8; 4] = b"UNGB";
const UNGEGN_OFFSETS: &[u8; 4] = b"UNGO";
const WORD_INDEX: &[u8; 4] = b"WIDX";
const KEY_INDEX: &[u8; 4] = b"KIDX";
const FORM_WORDS: &[u8; 4] = b"FWRD";
const FORM_SPELLINGS: &[u8; 4] = b"FSPL";
const FORM_SOURCES: &[u8; 4] = b"FSRC";
const SPELLING_BLOB: &[u8; 4] = b"SPLB";
const SPELLING_OFFSETS: &[u8; 4] = b"SPLO";
const BIGRAM_STARTS: &[u8; 4] = b"BGST";
const BIGRAM_NEXT: &[u8; 4] = b"BGNX";
const BIGRAM_COUNTS: &[u8; 4] = b"BGCT";
const FOLLOWING: &[u8; 4] = b"FOLL";
const SYLLABLE_INDEX: &[u8; 4] = b"SIDX";
const SYLLABLE_BLOB: &[u8; 4] = b"SYLB";
const SYLLABLE_OFFSETS: &[u8; 4] = b"SYLO";
const SYLLABLE_LOGPROBS: &[u8; 4] = b"SYLP";
const ENGLISH: &[u8; 4] = b"ENGL";

type Sections = Vec<([u8; 4], Vec<u8>)>;

/// Compile the files in `directory` (written by `data/export.py`) into one buffer.
pub fn compile(directory: &Path) -> Result<Vec<u8>, Error> {
    let read = |name: &str| tsv::read(&directory.join(name)).map_err(Error::from);
    let mut sections: Sections = Vec::new();
    let meta: String = read("meta.tsv")?
        .iter()
        .map(|row| [row[0].as_str(), "\t", row[1].as_str(), "\n"].concat())
        .collect();
    sections.push((*META, meta.into_bytes()));
    let words = read("words.tsv")?;
    compile_words(&words, &mut sections)?;
    compile_forms(&read("forms.tsv")?, &mut sections)?;
    compile_bigrams(&read("bigrams.tsv")?, words.len(), &mut sections)?;
    compile_syllables(&read("syllables.tsv")?, &mut sections)?;
    let text = std::fs::read_to_string(directory.join("english.txt"))?;
    let mut english: Vec<&str> = text.lines().filter(|w| !w.is_empty()).collect();
    english.sort_unstable();
    english.dedup();
    let mut builder = SetBuilder::memory();
    builder.extend_iter(english)?;
    sections.push((*ENGLISH, builder.into_inner()?));
    Ok(assemble(&sections))
}

fn compile_words(words: &[Vec<String>], sections: &mut Sections) -> Result<(), Error> {
    for (i, row) in words.iter().enumerate() {
        if row.len() != 6 || row[0] != i.to_string() {
            return Err(Error::new(format!("words.tsv: row {i} is malformed")));
        }
    }
    let column = |c: usize| words.iter().map(move |row| row[c].as_str());
    push_strings(sections, *WORD_BLOB, *WORD_OFFSETS, column(1));
    let counts: Result<Vec<u32>, _> = column(2).map(str::parse).collect();
    let counts = counts.map_err(|_| Error::new("words.tsv: bad count"))?;
    sections.push((*WORD_COUNTS, u32s(&counts)));
    sections.push((*WORD_KNOWN, column(3).map(|k| u8::from(k == "1")).collect()));
    push_strings(sections, *CHAT_BLOB, *CHAT_OFFSETS, column(4));
    push_strings(sections, *UNGEGN_BLOB, *UNGEGN_OFFSETS, column(5));
    let mut by_word: Vec<(&str, u64)> = column(1).zip(0u64..).collect();
    by_word.sort_unstable();
    by_word.dedup_by(|a, b| a.0 == b.0);
    sections.push((*WORD_INDEX, map_bytes(&by_word)?));
    Ok(())
}

/// Forms stay grouped by key in export order; the key index points at each key's run.
fn compile_forms(forms: &[Vec<String>], sections: &mut Sections) -> Result<(), Error> {
    let mut spellings: Vec<&str> = Vec::new();
    let mut spelling_ids: HashMap<&str, u32> = HashMap::new();
    let (mut words, mut spelling_column, mut sources) = (vec![], vec![], vec![]);
    let mut runs: HashMap<&str, (u64, u64)> = HashMap::new();
    for (i, row) in forms.iter().enumerate() {
        let word: u32 = row[1]
            .parse()
            .map_err(|_| Error::new("forms.tsv: bad word id"))?;
        let source = Source::from_name(&row[3])
            .ok_or_else(|| Error::new(format!("forms.tsv: unknown source {}", row[3])))?;
        let next = spellings.len() as u32;
        let spelling = *spelling_ids.entry(row[2].as_str()).or_insert_with(|| {
            spellings.push(row[2].as_str());
            next
        });
        words.push(word);
        spelling_column.push(spelling);
        sources.push(source as u8);
        let run = runs.entry(row[0].as_str()).or_insert((i as u64, 0));
        if run.0 + run.1 != i as u64 {
            return Err(Error::new(format!(
                "forms.tsv: key {} is not in one run",
                row[0]
            )));
        }
        run.1 += 1;
    }
    let mut keyed: Vec<(&str, u64)> = runs
        .into_iter()
        .map(|(k, (start, n))| (k, (start << 32) | n))
        .collect();
    keyed.sort_unstable();
    sections.push((*KEY_INDEX, map_bytes(&keyed)?));
    sections.push((*FORM_WORDS, u32s(&words)));
    sections.push((*FORM_SPELLINGS, u32s(&spelling_column)));
    sections.push((*FORM_SOURCES, sources));
    push_strings(
        sections,
        *SPELLING_BLOB,
        *SPELLING_OFFSETS,
        spellings.into_iter(),
    );
    Ok(())
}

/// Word pairs as a sparse matrix: each first word's pairs, sorted by second word.
fn compile_bigrams(
    rows: &[Vec<String>],
    words: usize,
    sections: &mut Sections,
) -> Result<(), Error> {
    let parse = |s: &str| {
        s.parse::<u32>()
            .map_err(|_| Error::new("bigrams.tsv: bad number"))
    };
    let mut pairs: Vec<(u32, u32, u32)> = Vec::with_capacity(rows.len());
    for row in rows {
        pairs.push((parse(&row[0])?, parse(&row[1])?, parse(&row[2])?));
    }
    pairs.sort_unstable();
    let mut starts = vec![0u32; words + 1];
    let mut following = vec![0u32; words];
    for &(first, _, count) in &pairs {
        starts[first as usize + 1] += 1;
        following[first as usize] += count;
    }
    for i in 0..words {
        starts[i + 1] += starts[i];
    }
    sections.push((*BIGRAM_STARTS, u32s(&starts)));
    sections.push((
        *BIGRAM_NEXT,
        u32s(&pairs.iter().map(|p| p.1).collect::<Vec<_>>()),
    ));
    sections.push((
        *BIGRAM_COUNTS,
        u32s(&pairs.iter().map(|p| p.2).collect::<Vec<_>>()),
    ));
    sections.push((*FOLLOWING, u32s(&following)));
    Ok(())
}

fn compile_syllables(rows: &[Vec<String>], sections: &mut Sections) -> Result<(), Error> {
    let mut keyed: Vec<(&str, u64)> = rows.iter().map(|row| row[0].as_str()).zip(0u64..).collect();
    keyed.sort_unstable();
    sections.push((*SYLLABLE_INDEX, map_bytes(&keyed)?));
    let spellings = rows.iter().map(|row| row[1].as_str());
    push_strings(sections, *SYLLABLE_BLOB, *SYLLABLE_OFFSETS, spellings);
    let logprobs: Result<Vec<f64>, _> = rows.iter().map(|row| row[2].parse::<f64>()).collect();
    let logprobs = logprobs.map_err(|_| Error::new("syllables.tsv: bad log probability"))?;
    sections.push((
        *SYLLABLE_LOGPROBS,
        logprobs.iter().flat_map(|p| p.to_le_bytes()).collect(),
    ));
    Ok(())
}

fn u32s(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn push_strings<'a>(
    sections: &mut Sections,
    blob_tag: [u8; 4],
    offsets_tag: [u8; 4],
    strings: impl Iterator<Item = &'a str>,
) {
    let mut blob = Vec::new();
    let mut offsets = vec![0u32];
    for s in strings {
        blob.extend_from_slice(s.as_bytes());
        offsets.push(blob.len() as u32);
    }
    sections.push((blob_tag, blob));
    sections.push((offsets_tag, u32s(&offsets)));
}

fn map_bytes(sorted: &[(&str, u64)]) -> Result<Vec<u8>, Error> {
    let mut builder = MapBuilder::memory();
    builder.extend_iter(sorted.iter().map(|&(k, v)| (k.as_bytes(), v)))?;
    Ok(builder.into_inner()?)
}

/// Header: magic, version, section count, then (tag, offset, length) per section.
fn assemble(sections: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let header = 12 + sections.len() * 20;
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(sections.len() as u32).to_le_bytes());
    let mut offset = header.next_multiple_of(8);
    for (tag, bytes) in sections {
        out.extend_from_slice(tag);
        out.extend_from_slice(&(offset as u64).to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        offset = (offset + bytes.len()).next_multiple_of(8);
    }
    for (_, bytes) in sections {
        out.resize(out.len().next_multiple_of(8), 0);
        out.extend_from_slice(bytes);
    }
    out
}

/// A shared view into the data buffer, so fst maps can read it without copying.
#[derive(Clone)]
struct Slice {
    bytes: Arc<[u8]>,
    start: usize,
    end: usize,
}

impl AsRef<[u8]> for Slice {
    fn as_ref(&self) -> &[u8] {
        &self.bytes[self.start..self.end]
    }
}

struct Strings {
    blob: Slice,
    offsets: Slice,
}

impl Strings {
    fn get(&self, i: usize) -> &str {
        let (start, end) = (
            u32_at(&self.offsets, i) as usize,
            u32_at(&self.offsets, i + 1) as usize,
        );
        // The compiler only writes UTF-8, and offsets fall on string boundaries.
        std::str::from_utf8(&self.blob.as_ref()[start..end]).unwrap_or("")
    }
}

fn u32_at(slice: &Slice, i: usize) -> u32 {
    let bytes = &slice.as_ref()[i * 4..i * 4 + 4];
    u32::from_le_bytes(bytes.try_into().expect("four bytes"))
}

/// The compiled data, read from one buffer without copying.
pub struct Data {
    pub settings: Settings,
    /// Corpus word count and vocabulary size, for the unigram probability.
    pub total: u64,
    pub vocabulary: u64,
    /// Every symbol that appears in a key, sorted, for the edit neighbourhood.
    pub alphabet: Vec<u8>,
    words: Strings,
    counts: Slice,
    known: Slice,
    chat: Strings,
    ungegn: Strings,
    word_index: Map<Slice>,
    key_index: Map<Slice>,
    form_words: Slice,
    form_spellings: Slice,
    form_sources: Slice,
    spellings: Strings,
    bigram_starts: Slice,
    bigram_next: Slice,
    bigram_counts: Slice,
    following: Slice,
    syllable_index: Map<Slice>,
    syllable_spellings: Strings,
    syllable_logprobs: Slice,
    english: Set<Slice>,
}

impl Data {
    /// Read a compiled data file.
    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::from_bytes(std::fs::read(path)?)
    }

    /// Read compiled data from a buffer.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let bytes: Arc<[u8]> = bytes.into();
        if bytes.len() < 12 || &bytes[..4] != MAGIC {
            return Err(Error::new("not a khmer-core data file"));
        }
        let field = |at: usize, width: usize| -> usize {
            let mut buffer = [0u8; 8];
            buffer[..width].copy_from_slice(&bytes[at..at + width]);
            u64::from_le_bytes(buffer) as usize
        };
        if field(4, 4) != VERSION as usize {
            return Err(Error::new(format!(
                "data version {} is not {VERSION}",
                field(4, 4)
            )));
        }
        let count = field(8, 4);
        let mut table: HashMap<[u8; 4], Slice> = HashMap::new();
        for i in 0..count {
            let at = 12 + i * 20;
            let tag: [u8; 4] = bytes[at..at + 4].try_into().expect("four bytes");
            let (start, length) = (field(at + 4, 8), field(at + 12, 8));
            if start + length > bytes.len() {
                return Err(Error::new("data file is truncated"));
            }
            table.insert(
                tag,
                Slice {
                    bytes: Arc::clone(&bytes),
                    start,
                    end: start + length,
                },
            );
        }
        let section = |tag: &[u8; 4]| {
            table.get(tag).cloned().ok_or_else(|| {
                Error::new(format!(
                    "data file has no {} section",
                    String::from_utf8_lossy(tag)
                ))
            })
        };
        let strings = |blob: &[u8; 4], offsets: &[u8; 4]| -> Result<Strings, Error> {
            Ok(Strings {
                blob: section(blob)?,
                offsets: section(offsets)?,
            })
        };

        let meta_slice = section(META)?;
        let meta_text =
            std::str::from_utf8(meta_slice.as_ref()).map_err(|_| Error::new("bad META"))?;
        let meta: HashMap<String, String> = meta_text
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let number = |name: &str| -> Result<u64, Error> {
            meta.get(name)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| Error::new(format!("meta: no {name}")))
        };
        Ok(Data {
            settings: Settings::from_meta(&meta)?,
            total: number("total")?,
            vocabulary: number("vocabulary")?,
            alphabet: meta
                .get("alphabet")
                .map(|a| a.as_bytes().to_vec())
                .unwrap_or_default(),
            words: strings(WORD_BLOB, WORD_OFFSETS)?,
            counts: section(WORD_COUNTS)?,
            known: section(WORD_KNOWN)?,
            chat: strings(CHAT_BLOB, CHAT_OFFSETS)?,
            ungegn: strings(UNGEGN_BLOB, UNGEGN_OFFSETS)?,
            word_index: Map::new(section(WORD_INDEX)?)?,
            key_index: Map::new(section(KEY_INDEX)?)?,
            form_words: section(FORM_WORDS)?,
            form_spellings: section(FORM_SPELLINGS)?,
            form_sources: section(FORM_SOURCES)?,
            spellings: strings(SPELLING_BLOB, SPELLING_OFFSETS)?,
            bigram_starts: section(BIGRAM_STARTS)?,
            bigram_next: section(BIGRAM_NEXT)?,
            bigram_counts: section(BIGRAM_COUNTS)?,
            following: section(FOLLOWING)?,
            syllable_index: Map::new(section(SYLLABLE_INDEX)?)?,
            syllable_spellings: strings(SYLLABLE_BLOB, SYLLABLE_OFFSETS)?,
            syllable_logprobs: section(SYLLABLE_LOGPROBS)?,
            english: Set::new(section(ENGLISH)?)?,
        })
    }

    /// How many words the data has (the lexicon plus any curated words outside it).
    pub fn len(&self) -> usize {
        self.counts.as_ref().len() / 4
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn word(&self, id: u32) -> &str {
        self.words.get(id as usize)
    }

    pub fn word_id(&self, word: &str) -> Option<u32> {
        self.word_index.get(word).map(|id| id as u32)
    }

    pub fn count(&self, id: u32) -> u32 {
        u32_at(&self.counts, id as usize)
    }

    /// Whether the word is in the lexicon (curated words may not be).
    pub fn known(&self, id: u32) -> bool {
        self.known.as_ref()[id as usize] == 1
    }

    pub fn chat(&self, id: u32) -> &str {
        self.chat.get(id as usize)
    }

    pub fn ungegn(&self, id: u32) -> &str {
        self.ungegn.get(id as usize)
    }

    /// Whether some word starts with `prefix` (for segmentation).
    pub fn has_word_prefix(&self, prefix: &str) -> bool {
        let mut stream = self.word_index.range().ge(prefix).into_stream();
        stream
            .next()
            .is_some_and(|(word, _)| word.starts_with(prefix.as_bytes()))
    }

    fn run(&self, packed: u64) -> impl Iterator<Item = Form<'_>> {
        let (start, length) = ((packed >> 32) as usize, (packed & 0xffff_ffff) as usize);
        (start..start + length).map(move |i| Form {
            word: u32_at(&self.form_words, i),
            spelling: self.spellings.get(u32_at(&self.form_spellings, i) as usize),
            source: Source::from_byte(self.form_sources.as_ref()[i]),
        })
    }

    /// The forms under one matching key, in the engine's order.
    pub fn forms(&self, key: &str) -> impl Iterator<Item = Form<'_>> {
        self.key_index
            .get(key)
            .into_iter()
            .flat_map(|packed| self.run(packed))
    }

    /// Keys that start with `prefix`, in byte order, at most `limit` of them.
    pub fn keys_with_prefix(&self, prefix: &str, limit: usize) -> Vec<(String, u64)> {
        let mut out = Vec::new();
        let mut stream = self.key_index.range().ge(prefix).into_stream();
        while let Some((key, packed)) = stream.next() {
            if out.len() >= limit || !key.starts_with(prefix.as_bytes()) {
                break;
            }
            out.push((String::from_utf8_lossy(key).into_owned(), packed));
        }
        out
    }

    /// The forms of a key found by [`Data::keys_with_prefix`].
    pub fn forms_at(&self, packed: u64) -> impl Iterator<Item = Form<'_>> {
        self.run(packed)
    }

    /// How often `second` followed `first` in the corpus.
    pub fn bigram(&self, first: u32, second: u32) -> u32 {
        let (start, end) = (
            u32_at(&self.bigram_starts, first as usize) as usize,
            u32_at(&self.bigram_starts, first as usize + 1) as usize,
        );
        let (mut low, mut high) = (start, end);
        while low < high {
            let middle = low.midpoint(high);
            match u32_at(&self.bigram_next, middle).cmp(&second) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return u32_at(&self.bigram_counts, middle),
            }
        }
        0
    }

    /// How many word pairs start with `first`, counted with their repeats.
    pub fn following(&self, first: u32) -> u32 {
        u32_at(&self.following, first as usize)
    }

    /// The most common spelling of a syllable key, and the key's log probability.
    pub fn syllable(&self, key: &str) -> Option<(&str, f64)> {
        let i = self.syllable_index.get(key)? as usize;
        let bytes = &self.syllable_logprobs.as_ref()[i * 8..i * 8 + 8];
        let logprob = f64::from_le_bytes(bytes.try_into().expect("eight bytes"));
        Some((self.syllable_spellings.get(i), logprob))
    }

    /// Whether a lowercase word is on the English list.
    pub fn is_english(&self, word: &str) -> bool {
        self.english.contains(word)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Data, Source, compile};

    fn sample() -> Data {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/sample");
        Data::from_bytes(compile(&directory).unwrap()).unwrap()
    }

    #[test]
    fn words_and_counts() {
        let data = sample();
        assert_eq!(data.len(), 3008);
        assert_eq!((data.word(0), data.count(0)), ("បាន", 136_888));
        let id = data.word_id("ខ្ញុំ").unwrap();
        assert_eq!(data.word(id), "ខ្ញុំ");
        assert!(data.known(id));
        assert_eq!(data.chat(id), "khnhom");
        assert!(data.word_id("not khmer").is_none());
    }

    #[test]
    fn forms_keep_the_engine_order() {
        let data = sample();
        let forms: Vec<_> = data
            .forms("bAn")
            .map(|f| (data.word(f.word), f.spelling, f.source))
            .collect();
        assert_eq!(forms[0], ("បាន", "ban", Source::Pronunciation));
        assert!(forms.len() > 1);
        assert_eq!(data.forms("no such key").count(), 0);
    }

    #[test]
    fn curated_forms() {
        let data = sample();
        let forms: Vec<_> = data.forms("ck").collect();
        assert!(
            forms
                .iter()
                .any(|f| f.source == Source::Curated && data.word(f.word) == "ចង់")
        );
    }

    #[test]
    fn prefixes() {
        let data = sample();
        let keys = data.keys_with_prefix("OkO", 100);
        assert!(keys.iter().any(|(k, _)| k == "OkOn"));
        assert!(keys.iter().all(|(k, _)| k.starts_with("OkO")));
        assert!(data.has_word_prefix("សុខ"));
        assert!(!data.has_word_prefix("xyz"));
    }

    #[test]
    fn bigrams() {
        let data = sample();
        let (first, second) = (data.word_id("នៅ").unwrap(), data.word_id("ក្នុង").unwrap());
        assert_eq!(data.bigram(first, second), 22_539);
        assert_eq!(data.bigram(second, first), 0);
        assert!(data.following(first) >= 22_539);
    }

    #[test]
    fn syllables_and_english() {
        let data = sample();
        let (spelling, logprob) = data.syllable("sO").unwrap();
        assert!(!spelling.is_empty() && logprob < 0.0);
        assert!(data.syllable("no such key").is_none());
        assert!(data.is_english("wifi"));
        assert!(!data.is_english("mean"));
    }

    #[test]
    fn settings_come_from_the_engine() {
        let data = sample();
        assert_eq!(data.settings.beam, 8);
        assert!((data.settings.key_edit - 5.0).abs() < f64::EPSILON);
        assert_eq!(data.vocabulary, 3008);
        assert_eq!(data.alphabet, b"AEJNOQYbcdfhklmnprstvyz");
    }

    #[test]
    fn rejects_other_files() {
        assert!(Data::from_bytes(b"not data at all".to_vec()).is_err());
    }
}
