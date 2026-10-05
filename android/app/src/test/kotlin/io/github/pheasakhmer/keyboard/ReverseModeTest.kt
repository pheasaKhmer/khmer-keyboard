package io.github.pheasakhmer.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

private object RomanizingCore : Core by NoCore {
    override fun romanize(khmer: String, ungegn: Boolean) = when (khmer) {
        "សុខសប្បាយទេ" -> if (ungegn) "sŏkhsâbbay té" else "soksabay te"
        "ទេ" -> "te"
        else -> khmer
    }
}

class ReverseModeTest {
    @Test
    fun selectedKhmerIsOfferedInChatStyleThenUngegn() {
        val bar = romanizations(RomanizingCore, "សុខសប្បាយទេ")!!
        assertEquals("សុខសប្បាយទេ", bar.typed)
        assertEquals("soksabay te", bar.best?.text)
        assertEquals(listOf("sŏkhsâbbay té"), bar.others.map { it.text })
        assertEquals(true, bar.best?.isLatin)
    }

    @Test
    fun theSameSpellingInBothStylesIsOfferedOnce() {
        assertEquals(emptyList<Candidate>(), romanizations(RomanizingCore, "ទេ")!!.others)
    }

    @Test
    fun selectionsWithoutKhmerOrTooLongAreLeftAlone() {
        assertNull(romanizations(RomanizingCore, "hello"))
        assertNull(romanizations(RomanizingCore, "ក".repeat(MAX_ROMANIZED_SELECTION + 1)))
    }
}
