package io.github.pheasakhmer.keyboard

/**
 * A reading of the text being typed, as the core gives it: characters [start] until [end]
 * of the typed text read as [text]. Latin readings (English words, or the text kept as
 * typed) are written with a space after them; Khmer is written without spaces.
 */
data class Reading(val text: String, val start: Int, val end: Int, val isLatin: Boolean)

/** What the keyboard needs from the core. */
interface Core {
    /** Up to [count] readings of the word at the end of [typed], best first. [context] is
     *  the text before it, whose last Khmer word helps choose between homophones. */
    fun suggest(context: String, typed: String, count: Int): List<Reading>

    /** The best conversion of [text]. */
    fun convert(text: String): String

    /** The user picked [word] for [typed]. */
    fun learn(typed: String, word: String)
}

/** A core that knows nothing yet, while the real one loads. */
object NoCore : Core {
    override fun suggest(context: String, typed: String, count: Int) = emptyList<Reading>()
    override fun convert(text: String) = text
    override fun learn(typed: String, word: String) {}
}

/** A candidate in the suggestion bar. */
data class Candidate(
    /** What is shown, and what is committed. */
    val text: String,
    val isLatin: Boolean,
    /** The typed letters the candidate reads, and the word learned for them if picked. */
    val typed: String,
    val word: String,
)

/** The suggestion bar's content: the typed text, the best candidate (what space commits)
 *  and the others, best first. */
data class Bar(val typed: String, val best: Candidate?, val others: List<Candidate>) {
    companion object {
        val EMPTY = Bar("", null, emptyList())
    }
}

/**
 * Turns keystrokes into candidates. It keeps the Latin text being typed (the editor's
 * composing text) and asks the core for readings after each change.
 *
 * The core reads the last word of what was typed, so a candidate may cover only its end:
 * typing "soksabayte" gives readings of "te". The rest is converted as a whole and put in
 * front, so every candidate is the full replacement for what was typed.
 */
class Composer(var core: Core = NoCore, private val count: Int = 8) {
    var typed: String = ""
        private set
    var bar: Bar = Bar.EMPTY
        private set

    /** Add [text] to what is being typed. [context] is the text before it in the field. */
    fun type(text: String, context: String) {
        typed += text
        refresh(context)
    }

    /** Remove the last typed character. */
    fun deleteLast(context: String) {
        typed = typed.dropLast(1)
        refresh(context)
    }

    /** What space commits: the best candidate, or null when nothing is being typed. */
    fun accept(): Candidate? {
        val best = bar.best ?: typed.takeIf { it.isNotEmpty() }?.let(::literal)
        reset()
        return best
    }

    /** The user tapped [candidate]. A Khmer pick is learned, so it ranks higher next time. */
    fun pick(candidate: Candidate): Candidate {
        if (!candidate.isLatin) core.learn(candidate.typed, candidate.word)
        reset()
        return candidate
    }

    /** Keep the typed text in Latin letters, as typed. */
    fun keepTyped(): Candidate = literal(typed).also { reset() }

    fun reset() {
        typed = ""
        bar = Bar.EMPTY
    }

    private fun literal(text: String) = Candidate(text, isLatin = true, typed = text, word = text)

    private fun refresh(context: String) {
        if (typed.isEmpty()) {
            bar = Bar.EMPTY
            return
        }
        val prefixes = HashMap<Int, String>()
        val candidates = LinkedHashMap<String, Candidate>()
        for (reading in core.suggest(context, typed, count)) {
            val start = reading.start.coerceIn(0, typed.length)
            val end = reading.end.coerceIn(start, typed.length)
            val prefix = if (start == 0) "" else prefixes.getOrPut(start) {
                core.convert(typed.substring(0, start))
            }
            val text = prefix + reading.text + typed.substring(end)
            candidates.putIfAbsent(
                text,
                Candidate(text, reading.isLatin, typed.substring(start, end), reading.text),
            )
        }
        val ranked = candidates.values.toList()
        bar = Bar(typed, ranked.firstOrNull(), ranked.drop(1))
    }
}
