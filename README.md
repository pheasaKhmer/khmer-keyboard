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
| 1. Shared core with tests and a desktop CLI | **done** |
| 2. Android keyboard, romanized mode | **done**: runs on an Android 17 emulator; not yet tried on a phone |
| 3. iOS keyboard | needs Xcode |
| 4. Khmer layout mode | **done** on Android: NiDA-based, switched with the ក key |
| 5. Reverse mode and learning in the apps | **done** on Android: select Khmer to romanize it; picks are learned by previous word |
| 6. Store listings, privacy policy | release build, privacy policy and listing text ready; graphics and the Play Console account to do |

## Layout

```
core/      khmer-core: the Rust library both keyboards call
ffi/       khmer-ffi: the core's Kotlin and Swift bindings, made by UniFFI
cli/       khmer-kbd: the core on the command line
data/      the export from khmer-engine, a sample, and the engine's golden answers
android/   the Android keyboard
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
- **learning**: picked candidates rank higher next time, most after the same previous word,
  so picking តេ for `te` after ចាំ does not push ទេ down everywhere
- **romanizing** Khmer back to Latin letters, in chat style or UNGEGN; words the lexicon
  does not know are spelled by the engine's rule-based romanizer, which the core ports

Golden tests hold the core to the engine: on the exported sample, all 18,254 matching keys,
294 conversions with their alternatives (including 65 phrases typed by a native speaker),
2,611 keystroke suggestion lists, 290 romanizations of text and 6,740 rule-based
romanizations of single words are identical to the Python engine's.

| Full lexicon (61,980 words) | |
|---|---|
| Data file | 10.4 MB (the spec allows 15 MB) |
| Load | 1.1 ms |
| `suggest` per keystroke, 5-word input | 0.36 ms median, 1.6 ms worst (desktop, release build) |

The spec's budget is 30 ms per keystroke on a mid-range Android phone.

### Known limits

- A word the lexicon does not know is romanized from its spelling by the engine's rules. They
  cannot know how the word is pronounced, so its chat spelling can differ from how people type
  it (ចក្រពត្តិ comes out "chakropotde").
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

From Kotlin and Swift, `khmer-ffi` wraps the same calls in one `Keyboard` object (`open` or
`fromBytes`, `suggest`, `learn`, `forget`, `convert`, `romanize`). Its bindings are generated
from the compiled library, so there is no interface file to keep in step.

## Android

The keyboard is an input method service with a QWERTY layout for romanized Khmer, a page of
digits and symbols, and a Khmer script layout after the NiDA standard keyboard (the ក key
switches to it, abc back). Letters are composed in the field, underlined, and the suggestion bar works
like the iPhone's:

- left: what you typed, in quotes; tap it to keep it in Latin letters
- middle: the best reading, which space commits
- right: the next reading, and an arrow that opens the rest (`te` can be ទេ, តែ, ទី or តេ)

A tapped reading is learned with the word before it and ranks higher next time, most
after that word. Selecting Khmer text shows it in Latin letters in the bar (reverse mode),
chat style in the middle and UNGEGN on the right; a tap replaces the selection. ។ sits next to space. In password,
email, web address and number fields the keys type what they show, and nothing is converted,
suggested or learned. The app has no permissions at all, so it cannot reach the network.

To build it you need Java 21, the Android SDK with platform 37, build tools 37.0.0 and NDK
29.0.14206865, the Android Rust targets and cargo-ndk:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
cd android && ./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
```

The build compiles the core for arm64-v8a, armeabi-v7a and x86_64 (Android 7 and up),
generates the Kotlin bindings, and packs `data/build` if it was exported, else `data/sample`.
Open the app to turn the keyboard on. The debug APK with the full lexicon is 14 MB.

### Release builds

A release needs the full lexicon in `data/build` (the build stops if it would pack the
sample) and is shrunk with R8. To sign it for the Play Store, create an upload key once,
keep it and its passwords safe, and never commit them:

```bash
cd android
keytool -genkeypair -v -keystore upload.jks -alias upload -keyalg RSA -keysize 4096 -validity 10000
```

Then write `android/keystore.properties` (ignored by git):

```
storeFile=upload.jks
storePassword=...
keyAlias=upload
keyPassword=...
```

and build the bundle to upload:

```bash
./gradlew bundleRelease    # app/build/outputs/bundle/release/app-release.aab, 6.6 MB
```

Without `keystore.properties`, release builds are signed with the debug key: fine for
installing and testing (`./gradlew assembleRelease`, 13.4 MB), refused by the Play Store.
[PRIVACY.md](PRIVACY.md) is the privacy policy, and
[android/store-listing.md](android/store-listing.md) drafts the store listing.

## Privacy

The core has no networking code. Learned picks stay in a file whose location the app chooses,
inside its own storage. Nothing typed is logged or sent anywhere.

## License

The code is [MIT](LICENSE). The data exported from khmer-engine keeps its sources' licenses:
CC BY 4.0 for the words and pronunciations, ODC-By 1.0 for the word counts (see
[data/README.md](data/README.md)).
