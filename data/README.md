# Data for the keyboard core

The keyboard core does not reimplement khmer-engine's lexicon romanizers. The engine
computes everything slow once, here, and the core compiles the exported files into one
compact file that it only looks things up in:

- the matching index: for each matching key, the words and romanizations it can be
- word counts, word-pair counts, and each word's chat and UNGEGN romanization
- the syllable table for words the lexicon does not know
- the English words that stay in Latin letters
- every weight the engine's scores use, so the core scores exactly like the engine

`export.py` documents each file's columns. The core does port the engine's rule-based
romanizer, which spells Khmer words the lexicon does not know.

## The sample

`sample/` is the export of the small lexicon bundled with khmer-engine (3,008 words). It is
committed so the core's tests run offline. `sample/golden/` holds the engine's own answers
for the inputs in `golden_inputs.txt` and `golden_khmer.txt`: matching keys, conversions,
suggestions after every keystroke, romanizations of text, and the rule-based romanization
of every lexicon word and of extra spellings. The core's tests check that it gives the same
answers.

Regenerate it after changing the engine pin in `pyproject.toml`:

```bash
cd data
uv run python export.py --out sample --golden
```

## The full lexicon

Build the full lexicon in a khmer-engine checkout (`make data`), then export it. The
output goes to `data/build/`, which is not committed:

```bash
cd data
uv run python export.py --lexicon ../../khmer-engine/data/build --out build
```

## Licenses

The export script is MIT, like the rest of this repository. The exported data keeps the
licenses of khmer-engine's sources:

- words, pronunciations and the romanizations derived from them: Google's Khmer
  pronunciation lexicon, Copyright 2018 Google Inc.,
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)
- word and word-pair counts: FineWeb-2's Khmer test split,
  [ODC-By 1.0](https://opendatacommons.org/licenses/by/1-0/)

See khmer-engine's
[data/README.md](https://github.com/pheasaKhmer/khmer-engine/blob/main/data/README.md) for
the details and the changes made to each source.
