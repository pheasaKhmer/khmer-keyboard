"""Export khmer-engine's index for the keyboard core, with golden test cases.

    uv run python export.py --out sample --golden       # the engine's bundled sample
    uv run python export.py --lexicon DIR --out build   # a full lexicon (khmer-engine: make data)

The Python engine does the slow work once: romanizing every word, building the matching
index and the syllable table, and tuning the weights. The Rust core compiles these files
into one compact file and only looks things up, so it gives the same results as the
engine without reimplementing the lexicon's romanizers. It ports only the rule-based
romanizer (khmer_engine.rules), for Khmer words the lexicon does not know.

Every file is UTF-8, tab-separated, with a comment line describing the columns. Text
fields escape backslash, tab and newline as \\\\, \\t and \\n.

    meta.tsv       name, value: corpus totals, the key alphabet, and every weight
    words.tsv      id, word, count, known (1 if in the lexicon), chat, ungegn
    forms.tsv      key, word id, spelling, source; each key's forms in the engine's order
    bigrams.tsv    first word id, second word id, count
    syllables.tsv  key, spelling, log probability
    english.txt    one word per line

With --golden, the engine's own answers for the inputs in golden_inputs.txt and
golden_khmer.txt, which the Rust tests must reproduce:

    golden/keys.tsv      text, final (1 or 0), key
    golden/convert.tsv   input, best, alternatives joined with " | "
    golden/suggest.tsv   input, suggestions as text|source|start|end joined with " ; "
    golden/romanize.tsv  khmer, style, output for every line of golden_khmer.txt
    golden/rules.tsv     word, style, rule-based romanization (khmer_engine.rules) of
                         every lexicon word, every word of golden_khmer.txt, the cases
                         in RULE_CASES, and every vowel and sign on ក and គ

The core normalizes Khmer with only part of pheasa (Khmer digits, zero-width spaces and
coeng da), so the export stops if a Khmer golden input needs more than that.
"""

import argparse
import dataclasses
import inspect
from pathlib import Path

import khmer_engine
from khmer_engine import decode, engine, match, transliterate
from khmer_engine.engine import Engine
from khmer_engine.keys import key
from khmer_engine.lexicon import Lexicon
from khmer_engine.romanize import merge_unknown
from khmer_engine.rules import romanize_word
from khmer_engine.script import COENG, DEPENDENT_VOWELS
from khmer_engine.segment import KHMER_RUN
from pheasa import normalize

HERE = Path(__file__).parent

# Words for golden/rules.tsv beyond the lexicon, separated by spaces: khmer-engine's tests
# of the rule-based romanizer, and spellings that exercise its rarer rules.
RULE_CASES = [
    # the engine's tests: province names, Wikipedia's examples, the UNGEGN report's notes
    "បន្ទាយ បាត់ដំបង កំពង់ចាម កំពង់ឆ្នាំង កំពង់ស្ពឺ កំពង់ធំ កំពត កណ្ដាល កោះកុង",
    "ក្រចេះ ភ្នំពេញ ព្រៃវែង ពោធិ៍សាត់ សៀមរាប ស្ទឹងត្រែង ស្វាយរៀង តាកែវ កែប ប៉ៃលិន",
    "ត្បូងឃ្មុំ ឧត្តរ ព្រះ មានជ័យ បន្ទាយមានជ័យ មណ្ឌលគិរី ឧត្តរមានជ័យ អក្សរខ្មែរ",
    "កម្ពុជា មណ្ឌល ពន្លឺ សន្តិភាព ជំនឿ ទៅ កក អង្គ ខ្ពង ល្អ ស្វាយ ក្ដី កន្ត្រាប់",
    "ក្អែក ចង្អៀត រអិល អ្វី អាង ខ្ពស់ រពាក់ ធម៌ បុណ្យ ពោធិ៍ ភូមិ ឯក ឪពុក ធ្វើ ពិះ",
    # subscript independent vowels
    "ហ្ឫទ័យ សុហ្ឫទ អម្ឫត ក្ឫ ហ្ឬ",
    # robat, on a final and on an onset
    "សួគ៌ កម៌ ធម៌ម អារ៌ក",
    # bantoc, on a final and on an onset
    "បត់ ចាក់ មាត់ ប្រាប់ ក់ក ក់ ចាក់ក",
    # muusikatoan and triisap
    "ហ៊ាង ញ៉ង ប៉ង ប៉ាតៅ ញ៉ាំ ម៉ោង ប៊ិច ប៉ុន្មាន ស៊ី អ៊ុំ",
    # samyok sannya
    "ច័ក ជ័យ វ័ង្គ ភ័ព្វ ទ័ព ស័ក្តិ",
    # toandakhiat, viriam, kakabat, ahsda, yuukaleapintu
    "ទេសចរណ៍ ព្រហ្ម៍ សម៑ នែ៎ ដ៏ សារៈ លេខ១",
    # splits, doubled consonants and finals with subscripts
    "សប្បាយ ទស្សនា បេក្ខជន ស្ត្រី សាស្ត្រ ចក្រពត្តិ ព្រហ្មញ្ញ យូធ្យូប",
    # text that is not a whole word: a vowel or a coeng without a base, joiners
    "ា ្ក ិក ក\u200cក ក\u200dក",
]


def escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def write(path: Path, header: str, rows: list[list[object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as out:
        out.write(f"# {header}\n")
        for row in rows:
            out.write("\t".join(escape(str(field)) for field in row) + "\n")


def settings() -> list[list[object]]:
    """Every number the engine's scores depend on, by name."""
    rows: list[list[object]] = []
    for prefix, values in (
        ("weights", match.Weights()),
        ("settings", decode.Settings()),
    ):
        for f in dataclasses.fields(values):
            rows.append([f"{prefix}.{f.name}", getattr(values, f.name)])
    weight = inspect.signature(Lexicon.bigram_logprob).parameters["weight"].default
    rows.append(["lexicon.bigram_weight", weight])
    for module, names in (
        (match, ["MIN_FUZZY_KEY", "MIN_COMPLETION_KEY", "MAX_COMPLETION_SCAN"]),
        (
            engine,
            [
                "FALLBACK_EMISSION",
                "TYPED_EMISSION",
                "LEARNED_WEIGHT",
                "LEARNED_ELSEWHERE_WEIGHT",
                "LEARNED_UNKNOWN_BONUS",
            ],
        ),
        (transliterate, ["MAX_SYLLABLE_LETTERS", "VOWEL_ONSET_COST"]),
    ):
        for name in names:
            rows.append([f"{module.__name__.rsplit('.', 1)[1]}.{name}", getattr(module, name)])
    return rows


def export_index(eng: Engine, out: Path) -> None:
    lexicon = eng.lexicon
    words = list(lexicon.entries)
    form_words = [f.word for forms in eng.matcher.index.values() for f in forms]
    words += [w for w in dict.fromkeys(form_words) if w not in lexicon.entries]
    ids = {w: i for i, w in enumerate(words)}

    meta = [
        ["engine", khmer_engine.__version__],
        ["total", lexicon.total],
        ["vocabulary", len(lexicon)],
        ["alphabet", "".join(eng.matcher.alphabet)],
        *settings(),
    ]
    write(out / "meta.tsv", "name\tvalue", meta)
    write(
        out / "words.tsv",
        "id\tword\tcount\tknown\tchat\tungegn",
        [
            [
                i,
                w,
                lexicon.entries[w].count if w in lexicon.entries else 0,
                int(w in lexicon.entries),
                eng.romanizer.word(w, "chat"),
                eng.romanizer.word(w, "ungegn"),
            ]
            for i, w in enumerate(words)
        ],
    )
    write(
        out / "forms.tsv",
        "key\tword id\tspelling\tsource",
        [
            [k, ids[f.word], f.spelling, f.source]
            for k, forms in eng.matcher.index.items()
            for f in forms
        ],
    )
    write(
        out / "bigrams.tsv",
        "first id\tsecond id\tcount",
        [[ids[a], ids[b], n] for (a, b), n in lexicon.bigrams.items()],
    )
    table = eng.transliterator
    write(
        out / "syllables.tsv",
        "key\tspelling\tlog probability",
        [[k, table.spelling[k], repr(table.logprob[k])] for k in table.spelling],
    )
    (out / "english.txt").write_text("\n".join(sorted(eng.english)) + "\n", encoding="utf-8")


def read_inputs(name: str) -> list[str]:
    lines = (HERE / name).read_text(encoding="utf-8").splitlines()
    return [line for line in lines if line and not line.startswith("#")]


_CORE_TABLE = {0x17E0 + d: str(d) for d in range(10)} | {0x200B: " "}


def core_normalize(text: str) -> str:
    """The part of pheasa's normalization the core has (core/src/segment.rs)."""
    return text.translate(_CORE_TABLE).replace(COENG + "ដ", COENG + "ត")


def check_normalized(texts: list[str]) -> None:
    """Stop if the core would normalize a golden input differently from the engine."""
    differ = [t for t in texts if normalize(t, digits="ascii", zwsp="space") != core_normalize(t)]
    if differ:
        raise SystemExit(f"golden inputs the core cannot normalize like pheasa: {differ}")


def rule_words(eng: Engine, khmer: list[str]) -> list[str]:
    """Words for golden/rules.tsv: the lexicon, each word of the golden Khmer text as the
    romanizer segments it, RULE_CASES, and every vowel and sign on an a-series and an
    o-series consonant, open and before the finals that change how they are read."""
    words = list(eng.lexicon.entries)
    for text in khmer:
        text = normalize(text, digits="ascii", zwsp="space")
        for run in KHMER_RUN.findall(text):
            words += [w.text for w in merge_unknown(eng.romanizer.segmenter.segment(run))]
    words += ["", *(word for line in RULE_CASES for word in line.split())]
    nuclei = [*sorted(DEPENDENT_VOWELS), "ំ", "ុំ", "ាំ", "ះ", "ុះ", "េះ", "ោះ", "ិះ", "ើះ", "ៈ", "័"]
    for base in "កគ":
        for nucleus in ["", *nuclei]:
            words += [base + nucleus + final for final in ("", "ក", "ង", "យ", "ន", "ក់")]
    return list(dict.fromkeys(words))


def export_golden(eng: Engine, out: Path) -> None:
    inputs = read_inputs("golden_inputs.txt")
    texts = list(dict.fromkeys(inputs + [w for line in inputs for w in line.split()]))
    texts += [f.spelling for forms in eng.matcher.index.values() for f in forms]
    texts += ["", "a", "Kâmpŭchéa", "l'â", "sabai", "orkun", "dar", "preah", "123", "œ"]
    texts = list(dict.fromkeys(texts))
    write(
        out / "golden" / "keys.tsv",
        "text\tfinal\tkey",
        [[t, int(final), key(t, final)] for t in texts for final in (True, False)],
    )

    convert = []
    suggest = []
    for text in inputs:
        result = eng.analyze(text, 5)
        convert.append([text, result.text, " | ".join(result.alternatives)])
        prefixes = [text[:end] for end in range(1, len(text) + 1)] if len(text) <= 24 else [text]
        for prefix in prefixes:
            found = eng.suggest(prefix, 5)
            suggest.append(
                [prefix, " ; ".join(f"{s.text}|{s.source}|{s.start}|{s.end}" for s in found)]
            )
    write(out / "golden" / "convert.tsv", "input\tbest\talternatives", convert)
    write(
        out / "golden" / "suggest.tsv",
        "input\tsuggestions",
        list({r[0]: r for r in suggest}.values()),
    )

    khmer = read_inputs("golden_khmer.txt")
    words = rule_words(eng, khmer)
    check_normalized(khmer + words)
    styles = ("chat", "ungegn")
    romanize = [[text, style, eng.romanize(text, style)] for text in khmer for style in styles]
    write(out / "golden" / "romanize.tsv", "khmer\tstyle\toutput", romanize)
    rules = [[word, style, romanize_word(word, style)] for word in words for style in styles]
    write(out / "golden" / "rules.tsv", "word\tstyle\tromanization", rules)
    print(f"golden: {len(texts) * 2} keys, {len(convert)} conversions, {len(suggest)} suggestions,")
    print(f"        {len(romanize)} romanizations, {len(rules)} rule-based romanizations")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--lexicon", type=Path, help="lexicon directory (default: the sample)")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--golden", action="store_true", help="also write golden test cases")
    args = parser.parse_args()
    eng = Engine(Lexicon.load(args.lexicon) if args.lexicon else None)
    export_index(eng, args.out)
    if args.golden:
        export_golden(eng, args.out)
    sizes = sum(p.stat().st_size for p in args.out.rglob("*") if p.is_file())
    print(f"wrote {args.out} ({sizes / 1024:.0f} KB), {len(eng.lexicon):,} words")


if __name__ == "__main__":
    main()
