# khmer-keyboard

An iOS and Android keyboard that turns romanized Khmer into Khmer script as you type.

Type `sok sabay` with Latin letters and pick សុខសប្បាយ from the suggestion bar. The keyboard
will also have a standard Khmer layout for typing Khmer directly, and a reverse mode that turns
Khmer script back into romanized text.

Everything runs on the device. The keyboard has no network access, and nothing you type leaves
your phone.

## Status

| Milestone | State |
|---|---|
| 1. Shared core with tests and a desktop CLI | **done** (this repository) |
| 2. Android keyboard, romanized mode | next; needs Android Studio and a JDK |
| 3. iOS keyboard | needs Xcode |
| 4. Khmer layout mode | |
| 5. Reverse mode and learning in the apps | the core already learns and romanizes known words |
| 6. Store listings, privacy policy | |

## Layout

```
core/      khmer-core: the Rust library both keyboards will call
cli/       khmer-kbd: the core on the command line
data/      the export from khmer-engine, a sample, and the engine's golden answers
android/   the Android keyboard (milestone 2)
ios/       the iOS keyboard (milestone 3)
```

## How the core works

The conversion is [khmer-engine](https://github.com/pheasaKhmer/khmer-engine)'s, ported to Rust.
The engine does everything slow once, at build time: it romanizes every word, builds the index of
romanizations, computes the syllable table and tunes the weights. `data/export.py` writes all of
that as plain files. The core compiles them into one read-only data file and only looks things
up and runs the search:

- **matching keys** fold chat spellings together ("sous dey" = "suosdey" = "sursdey"),
  plus every key one edit away
- **abbreviations** the way a native speaker types them: common words by their consonants
  (`tv` ទៅ, `dg` ដឹង), words without an unstressed first vowel (`sbay` សប្បាយ), and a
  curated list (`nh` ខ្ញុំ, `hz` ហើយ)
- **scoring** by key edits, letter distance, word frequency and curated spellings
- **a Viterbi search** over word sequences with word-pair probabilities, so context picks
  between words that sound alike (បង / បង់), with words typed across spaces ("or kun") or
  without them ("soksabayte")
- English words stay as typed; unknown words are spelled syllable by syllable; a word
  typed twice is written once with ៗ (`ban hz hz` → បានហើយៗ)
- **learning**: picked candidates rank higher next time
- **romanizing** Khmer back to Latin letters, in chat style or UNGEGN

Golden tests hold the core to the engine: on the exported sample, all 18,254 matching keys,
294 conversions with their alternatives (including 65 phrases typed by a native speaker),
2,611 keystroke suggestion lists and 262 romanizations are identical to the Python
engine's.

| Full lexicon (61,980 words) | |
|---|---|
| Data file | 10.4 MB (the spec allows 15 MB) |
| Load | 1.1 ms |
| `suggest` per keystroke, 5-word input | 0.36 ms median, 1.6 ms worst (desktop, release build) |

The spec's budget is 30 ms per keystroke on a mid-range Android phone.

### Known limits

- Romanizing a word the lexicon does not know leaves it in Khmer script. Spelling it needs a port
  of the engine's rule-based romanizers, planned with reverse mode.
- Input normalization covers Khmer digits, zero-width spaces and coeng da / coeng ta. Reordering
  marks typed out of order, which [pheasa](https://github.com/pheasaKhmer/pheasa) also does, is
  not ported yet.

## Build and try

You need Rust (`rust-toolchain.toml` pins the version; [rustup](https://rustup.rs/) installs it).

```bash
make check                                   # rustfmt, clippy, all tests
cargo run -p khmer-kbd -- convert "sok sabay te"
cargo run -p khmer-kbd -- suggest --context "ការ" bong
cargo run -p khmer-kbd -- romanize --style ungegn "សុខសប្បាយទេ"
cargo run -p khmer-kbd -- repl
```

By default the CLI uses `data/sample` (3,009 words). For the full lexicon, build it in a
khmer-engine checkout (`make data`), then export and compile it:

```bash
cd data && uv run python export.py --lexicon ../../khmer-engine/data/build --out build && cd ..
cargo run --release -p khmer-kbd -- compile data/build khmer.kbd
cargo run --release -p khmer-kbd -- --data khmer.kbd repl
```

## The core's API

```rust
use khmer_core::{Data, Engine, Style, UserDictionary};

let data = Data::open(Path::new("khmer.kbd"))?;
let mut engine = Engine::with_user(data, UserDictionary::open(Path::new("picks.tsv"))?);

// Candidates for the word being typed, after what is already in the field.
let suggestions = engine.suggest_in_context("ការ", "bong", 5);
// Each suggestion says which characters of the typed text it replaces.
let first = &suggestions[0];
engine.learn("bong", &first.text)?;   // the spec's commit(choice)
engine.romanize("សុខសប្បាយទេ", Style::Chat); // "soksabay te"
engine.forget()?;                     // the settings screen's "clear learned words"
```

Bindings for Swift and Kotlin (UniFFI) come with the first app.

## Privacy

The core has no networking code. Learned picks stay in a file whose location the app chooses,
inside its own storage. Nothing typed is logged or sent anywhere.

## License

The code is [MIT](LICENSE). The data exported from khmer-engine keeps its sources' licenses:
CC BY 4.0 for the words and pronunciations, ODC-By 1.0 for the word counts (see
[data/README.md](data/README.md)).
