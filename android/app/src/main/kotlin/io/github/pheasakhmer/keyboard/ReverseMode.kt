package io.github.pheasakhmer.keyboard

/** Longer selections are left alone: the bar shows one line. */
const val MAX_ROMANIZED_SELECTION = 2000

/**
 * Reverse mode: for selected Khmer text, a bar offering it in Latin letters, the chat style
 * first (the best, which a tap commits in place of the selection) and then UNGEGN. Null when
 * the selection has no Khmer or is too long.
 */
fun romanizations(core: Core, selected: String): Bar? {
    if (selected.length > MAX_ROMANIZED_SELECTION || selected.none(::isKhmer)) return null
    val offers = listOf(core.romanize(selected, ungegn = false), core.romanize(selected, ungegn = true))
    val candidates = offers.distinct().filter { it.isNotBlank() && it != selected }
        .map { Candidate(it, isLatin = true, typed = selected, word = it) }
    if (candidates.isEmpty()) return null
    return Bar(selected, candidates.first(), candidates.drop(1))
}

private fun isKhmer(ch: Char) = ch in 'ក'..'៿'
