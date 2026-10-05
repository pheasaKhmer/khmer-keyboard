package io.github.pheasakhmer.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** A core with fixed answers, recording what was learned. */
private class FakeCore(
    private val readings: Map<String, List<Reading>>,
    private val conversions: Map<String, String> = emptyMap(),
) : Core {
    val learned = mutableListOf<Triple<String, String, String>>()
    val contexts = mutableListOf<String>()

    override fun suggest(context: String, typed: String, count: Int): List<Reading> {
        contexts += context
        return readings[typed].orEmpty().take(count)
    }

    override fun convert(text: String) = conversions[text] ?: text

    override fun learn(context: String, typed: String, word: String) {
        learned += Triple(context, typed, word)
    }
}

class ComposerTest {
    private val te = listOf(
        Reading("ទេ", 0, 2, isLatin = false),
        Reading("តែ", 0, 2, isLatin = false),
        Reading("តេ", 0, 2, isLatin = false),
    )

    @Test
    fun typingAsksTheCoreWithTheContext() {
        val core = FakeCore(mapOf("t" to emptyList(), "te" to te))
        val composer = Composer(core)
        composer.type("t", "ចាំ")
        composer.type("e", "ចាំ")
        assertEquals("te", composer.bar.typed)
        assertEquals("ទេ", composer.bar.best?.text)
        assertEquals(listOf("តែ", "តេ"), composer.bar.others.map { it.text })
        assertEquals(listOf("ចាំ", "ចាំ"), core.contexts)
    }

    @Test
    fun spaceCommitsTheBestCandidateWithoutLearning() {
        val core = FakeCore(mapOf("te" to te))
        val composer = Composer(core)
        composer.type("te", "")
        assertEquals("ទេ", composer.accept()?.text)
        assertEquals("", composer.typed)
        assertEquals(emptyList<Triple<String, String, String>>(), core.learned)
        assertNull(composer.accept())
    }

    @Test
    fun aPickedCandidateIsLearned() {
        val core = FakeCore(mapOf("te" to te))
        val composer = Composer(core)
        composer.type("te", "ចាំ")
        val picked = composer.pick(composer.bar.others[1])
        assertEquals("តេ", picked.text)
        assertEquals(listOf(Triple("ចាំ", "te", "តេ")), core.learned)
    }

    @Test
    fun keepingTheTypedTextIsLatinAndNotLearned() {
        val core = FakeCore(mapOf("ok" to listOf(Reading("អូខេ", 0, 2, isLatin = false))))
        val composer = Composer(core)
        composer.type("ok", "")
        val kept = composer.keepTyped()
        assertEquals(Candidate("ok", isLatin = true, typed = "ok", word = "ok"), kept)
        assertEquals(emptyList<Triple<String, String, String>>(), core.learned)
    }

    @Test
    fun aReadingOfTheLastWordGetsTheRestConvertedInFront() {
        val core = FakeCore(
            mapOf("soksabayte" to listOf(Reading("ទេ", 8, 10, isLatin = false))),
            mapOf("soksabay" to "សុខសប្បាយ"),
        )
        val composer = Composer(core)
        composer.type("soksabayte", "ខ្ញុំ")
        val best = composer.bar.best!!
        assertEquals("សុខសប្បាយទេ", best.text)
        assertEquals("te" to "ទេ", best.typed to best.word)
        // A pick of it is learned after the converted words in front.
        composer.pick(best)
        assertEquals(listOf(Triple("ខ្ញុំសុខសប្បាយ", "te", "ទេ")), core.learned)
    }

    @Test
    fun withoutReadingsSpaceKeepsTheTypedText() {
        val composer = Composer(NoCore)
        composer.type("xyz", "")
        assertNull(composer.bar.best)
        assertEquals(Candidate("xyz", true, "xyz", "xyz"), composer.accept())
    }

    @Test
    fun deletingTheLastLetterEmptiesTheBar() {
        val composer = Composer(FakeCore(mapOf("t" to listOf(Reading("ត", 0, 1, false)))))
        composer.type("t", "")
        composer.deleteLast("")
        assertEquals(Bar.EMPTY, composer.bar)
    }

    @Test
    fun duplicateCandidatesAreShownOnce() {
        val core = FakeCore(
            mapOf("bong" to listOf(Reading("បង", 0, 4, false), Reading("បង", 0, 4, false))),
        )
        val composer = Composer(core)
        composer.type("bong", "")
        assertEquals(emptyList<Candidate>(), composer.bar.others)
    }
}
