"""Export khmer-engine's index for the keyboard core, with golden test cases.

    uv run python export.py --out sample --golden       # the engine's bundled sample
    uv run python export.py --lexicon DIR --out build   # a full lexicon (khmer-engine: make data)

The Python engine does the slow work once: romanizing every word, building the matching
index and the syllable table, and tuning the weights. The Rust core compiles these files
into one compact file and only looks things up, so it gives the same results as the
engine without reimplementing the romanizers.

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
    golden/romanize.tsv  khmer, style, output (only text whose words are all known)
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
from khmer_engine.segment import KHMER_RUN

HERE = Path(__file__).parent


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
            ["FALLBACK_EMISSION", "TYPED_EMISSION", "LEARNED_WEIGHT", "LEARNED_UNKNOWN_BONUS"],
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

    romanize = []
    for khmer in read_inputs("golden_khmer.txt"):
        runs = KHMER_RUN.findall(khmer)
        pieces = [w for run in runs for w in merge_unknown(eng.romanizer.segmenter.segment(run))]
        if all(w.known for w in pieces):
            for style in ("chat", "ungegn"):
                romanize.append([khmer, style, eng.romanize(khmer, style)])
    write(out / "golden" / "romanize.tsv", "khmer\tstyle\toutput", romanize)
    print(f"golden: {len(texts) * 2} keys, {len(convert)} conversions, {len(suggest)} suggestions,")
    print(f"        {len(romanize)} romanizations")


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
