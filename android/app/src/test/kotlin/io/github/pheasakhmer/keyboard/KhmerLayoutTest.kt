package io.github.pheasakhmer.keyboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class KhmerLayoutTest {
    @Test
    fun shiftedRowsMatchTheRowsKeyForKey() {
        assertEquals(KhmerLayout.rows.map { it.size }, KhmerLayout.shifted.map { it.size })
    }

    @Test
    fun everyKeyTypesKhmer() {
        val keys = (KhmerLayout.rows + KhmerLayout.shifted + KhmerLayout.symbols).flatten()
        val others = keys.filterNot { key -> key.all { it in 'ក'..'៿' } }
        assertEquals(listOf("?", "«", "»"), others)
    }

    @Test
    fun theTopRowHoldsTheKhmerDigits() {
        assertTrue(KhmerLayout.rows[0].size >= KhmerLayout.DIGITS.length)
    }

    @Test
    fun marksAreShownOnADottedCircle() {
        assertEquals("◌ា", KhmerLayout.label("ា"))
        assertEquals("◌្", KhmerLayout.label("្"))
        assertEquals("◌ុំ", KhmerLayout.label("ុំ"))
        assertEquals("ក", KhmerLayout.label("ក"))
        assertEquals("ឪ", KhmerLayout.label("ឪ"))
    }
}
