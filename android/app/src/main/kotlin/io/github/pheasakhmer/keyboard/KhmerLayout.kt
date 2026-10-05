package io.github.pheasakhmer.keyboard

/**
 * The Khmer script layout, after the NiDA standard keyboard (the "Khmer (NIDA)" layout on
 * computers): its three letter rows, unshifted and shifted, as three rows on a phone. Keys
 * type their text as it is; a key may type more than one character (ាំ, ោះ).
 */
object KhmerLayout {
    val rows: List<List<String>> = listOf(
        listOf("ឆ", "ឹ", "េ", "រ", "ត", "យ", "ុ", "ិ", "ោ", "ផ", "ៀ", "ឪ"),
        listOf("ា", "ស", "ដ", "ថ", "ង", "ហ", "្", "ក", "ល", "ើ", "់"),
        listOf("ឋ", "ខ", "ច", "វ", "ប", "ន", "ម", "ុំ", "។", "៊"),
    )

    val shifted: List<List<String>> = listOf(
        listOf("ឈ", "ឺ", "ែ", "ឬ", "ទ", "ួ", "ូ", "ី", "ៅ", "ភ", "ឿ", "ឧ"),
        listOf("ាំ", "ៃ", "ឌ", "ធ", "អ", "ះ", "ញ", "គ", "ឡ", "ោះ", "៉"),
        listOf("ឍ", "ឃ", "ជ", "េះ", "ព", "ណ", "ំ", "ុះ", "៕", "?"),
    )

    /** Typed by holding the first ten keys of the top row. */
    const val DIGITS = "១២៣៤៥៦៧៨៩០"

    /** The page behind ១២៣: digits, then the independent vowels and signs NiDA puts on the
     *  number row or that the letter rows have no room for. */
    val symbols: List<List<String>> = listOf(
        DIGITS.map(Char::toString),
        listOf("ៗ", "៛", "ឥ", "ឦ", "ឧ", "ឩ", "ឯ", "ឲ", "ឮ", "ឭ"),
        listOf("៍", "័", "៏", "៌", "៎", "៑", "៖", "«", "»"),
    )

    /** How a key shows [text]: a vowel sign or other mark on its own is drawn on a dotted
     *  circle, as Khmer keyboards and fonts show it. */
    fun label(text: String): String = if (isMark(text.first())) "◌$text" else text

    private fun isMark(ch: Char): Boolean = Character.getType(ch).toByte().let {
        it == Character.NON_SPACING_MARK || it == Character.COMBINING_SPACING_MARK
    }
}
