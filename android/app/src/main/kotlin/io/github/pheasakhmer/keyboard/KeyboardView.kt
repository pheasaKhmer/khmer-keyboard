package io.github.pheasakhmer.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsets
import android.widget.LinearLayout
import android.widget.TextView

/** What the keys do; the input method service implements it. */
interface KeyListener {
    /** A letter, which is composed. */
    fun onLetter(letter: String)

    /** A digit, punctuation or symbol, which ends the word being typed. */
    fun onSymbol(symbol: String)

    fun onSpace()
    fun onDelete()
    fun onEnter()
    fun onSwitchKeyboard()
    fun onPick(candidate: Candidate)
    fun onKeepTyped()
}

/**
 * The keyboard: the suggestion bar over a QWERTY layout for romanized Khmer, and a page of
 * digits and symbols. The bottom row has ។ next to space, since Khmer ends sentences
 * with it.
 */
@SuppressLint("ViewConstructor")
class KeyboardView(context: Context, private val listener: KeyListener) : LinearLayout(context) {
    private val palette = Palette(context)
    val bar = SuggestionBarView(context, palette, listener::onPick, listener::onKeepTyped)
    private val keys = LinearLayout(context).apply { orientation = VERTICAL }
    private val repeat = Handler(Looper.getMainLooper())

    private var symbols = false
    private var shifted = false
    private var showSwitchKey = true
    private var enterLabel = "⏎"

    init {
        orientation = VERTICAL
        setBackgroundColor(palette.background)
        addView(bar, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
        addView(keys, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
        val gap = palette.dp(3f)
        keys.setPadding(gap, 0, gap, palette.dp(6f))
        // From Android 15 the keyboard is drawn under the navigation bar; keep the keys
        // above it.
        setOnApplyWindowInsetsListener { _, insets ->
            val bottom = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                insets.getInsets(WindowInsets.Type.navigationBars()).bottom
            } else {
                @Suppress("DEPRECATION")
                insets.systemWindowInsetBottom
            }
            keys.setPadding(gap, 0, gap, palette.dp(6f) + bottom)
            insets
        }
        layoutKeys()
    }

    /** Set up for a new field: letters, no shift, and the field's enter label. */
    fun reset(enterLabel: String, showSwitchKey: Boolean, startWithSymbols: Boolean) {
        this.enterLabel = enterLabel
        this.showSwitchKey = showSwitchKey
        symbols = startWithSymbols
        shifted = false
        layoutKeys()
    }

    private fun layoutKeys() {
        keys.removeAllViews()
        if (symbols) {
            row("1234567890".map { symbolKey(it.toString()) })
            row("-/:;()៛&@\"".map { symbolKey(it.toString()) })
            row(".,?!'%+=".map { symbolKey(it.toString()) } + deleteKey())
        } else {
            row("qwertyuiop".map { letterKey(it) })
            row(listOf(spacer(0.5f)) + "asdfghjkl".map { letterKey(it) } + spacer(0.5f))
            row(listOf(shiftKey()) + "zxcvbnm".map { letterKey(it) } + deleteKey())
        }
        val bottom = mutableListOf(
            key(if (symbols) "ABC" else "123", 1.5f, special = true) {
                symbols = !symbols
                layoutKeys()
            },
        )
        if (showSwitchKey) {
            bottom += key("🌐", 1f, special = true, label = R.string.switch_keyboard) {
                listener.onSwitchKeyboard()
            }
        }
        bottom += key(context.getString(R.string.space), 4.5f) { listener.onSpace() }
        bottom += key("។", 1f) { listener.onSymbol("។") }
        bottom += key(enterLabel, 1.5f, special = true, label = R.string.enter) { listener.onEnter() }
        row(bottom)
    }

    private fun row(views: List<View>) {
        val row = LinearLayout(context).apply { orientation = HORIZONTAL }
        for (view in views) row.addView(view)
        keys.addView(row, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, palette.dp(52f)))
    }

    private fun letterKey(letter: Char): View {
        val shown = if (shifted) letter.uppercaseChar() else letter
        return key(shown.toString(), 1f) {
            listener.onLetter(shown.toString())
            if (shifted) {
                shifted = false
                layoutKeys()
            }
        }
    }

    private fun symbolKey(symbol: String) = key(symbol, 1f) { listener.onSymbol(symbol) }

    private fun shiftKey() = key(if (shifted) "⬆" else "⇧", 1.5f, special = true, label = R.string.shift) {
        shifted = !shifted
        layoutKeys()
    }

    /** Delete repeats while held, after a short wait. */
    @SuppressLint("ClickableViewAccessibility")
    private fun deleteKey(): View {
        val view = key("⌫", 1.5f, special = true, label = R.string.delete) { listener.onDelete() }
        val tick = object : Runnable {
            override fun run() {
                listener.onDelete()
                repeat.postDelayed(this, REPEAT_INTERVAL)
            }
        }
        view.setOnTouchListener { v, event ->
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    v.isPressed = true
                    v.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    listener.onDelete()
                    repeat.postDelayed(tick, REPEAT_DELAY)
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                    v.isPressed = false
                    repeat.removeCallbacks(tick)
                }
            }
            true
        }
        return view
    }

    private fun spacer(weight: Float) = View(context).apply {
        layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, weight)
    }

    private fun key(
        text: String,
        weight: Float,
        special: Boolean = false,
        label: Int? = null,
        onTap: () -> Unit,
    ): View = TextView(context).apply {
        this.text = text
        gravity = Gravity.CENTER
        textSize = if (text.length > 1) 16f else 22f
        setTextColor(palette.text)
        background = palette.keyBackground(if (special) palette.special else palette.key)
        label?.let { contentDescription = context.getString(it) }
        isSoundEffectsEnabled = true
        setOnClickListener {
            it.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
            onTap()
        }
        val gap = palette.dp(3f)
        layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, weight).apply {
            setMargins(gap, gap, gap, gap)
        }
    }

    override fun onDetachedFromWindow() {
        repeat.removeCallbacksAndMessages(null)
        super.onDetachedFromWindow()
    }

    private companion object {
        const val REPEAT_DELAY = 400L
        const val REPEAT_INTERVAL = 50L
    }
}
