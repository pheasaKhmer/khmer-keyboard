package io.github.pheasakhmer.keyboard

import android.content.Context
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.StateListDrawable
import android.util.TypedValue

/** The keyboard's colours, following the phone's light or dark setting. */
class Palette(context: Context) {
    private val dark = context.resources.configuration.uiMode and
        Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    private val density = context.resources.displayMetrics.density

    val background = if (dark) Color.rgb(0x20, 0x21, 0x24) else Color.rgb(0xd1, 0xd4, 0xd9)
    val key = if (dark) Color.rgb(0x46, 0x47, 0x4b) else Color.WHITE
    val keyPressed = if (dark) Color.rgb(0x5f, 0x60, 0x65) else Color.rgb(0xe4, 0xe6, 0xea)
    val special = if (dark) Color.rgb(0x31, 0x32, 0x36) else Color.rgb(0xab, 0xb0, 0xba)
    val text = if (dark) Color.WHITE else Color.BLACK
    val muted = if (dark) Color.rgb(0xa8, 0xa9, 0xad) else Color.rgb(0x5f, 0x63, 0x68)
    val divider = if (dark) Color.rgb(0x3c, 0x3d, 0x41) else Color.rgb(0xb8, 0xbb, 0xc2)

    fun dp(value: Float): Int = (value * density + 0.5f).toInt()

    fun sp(context: Context, value: Float): Float =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value, context.resources.displayMetrics)

    /** A rounded key background that darkens while pressed. */
    fun keyBackground(color: Int): StateListDrawable = StateListDrawable().apply {
        addState(intArrayOf(android.R.attr.state_pressed), rounded(keyPressed))
        addState(intArrayOf(), rounded(color))
    }

    private fun rounded(color: Int) = GradientDrawable().apply {
        cornerRadius = dp(6f).toFloat()
        setColor(color)
    }
}
