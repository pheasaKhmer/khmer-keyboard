package io.github.pheasakhmer.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.FrameLayout.LayoutParams as FrameParams
import android.widget.LinearLayout.LayoutParams as LinearParams

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

    /** Show the system's list of keyboards (a long press on the globe). */
    fun onChooseKeyboard()
    fun onPick(candidate: Candidate)
    fun onKeepTyped()
}

/**
 * The keyboard: the suggestion bar over a QWERTY layout for romanized Khmer, and a page of
 * digits and symbols. The bottom row has ។ next to space, since Khmer ends sentences
 * with it.
 *
 * Pressing a character key shows it enlarged above the finger. Holding a key on the top
 * row types its digit (shown small in the corner), and holding ។ types ៕. Shift tapped
 * twice quickly stays on.
 */
@SuppressLint("ViewConstructor")
class KeyboardView(context: Context, private val listener: KeyListener) : FrameLayout(context) {
    private enum class Shift { OFF, ONCE, LOCKED }

    private val palette = Palette(context)
    private val keyHeight = palette.dp(52f)
    private val gap = palette.dp(3f)
    val bar = SuggestionBarView(context, palette, listener::onPick, listener::onKeepTyped)
    private val keys = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
    private val preview = TextView(context).apply {
        gravity = Gravity.CENTER
        textSize = 28f
        setTextColor(palette.text)
        background = palette.keyBackground(palette.key)
        elevation = palette.dp(4f).toFloat()
        visibility = GONE
    }
    private val handler = Handler(Looper.getMainLooper())
    private val hidePreview = Runnable { preview.visibility = GONE }

    private var symbols = false
    private var shift = Shift.OFF
    private var lastShiftTap = 0L
    private var showSwitchKey = true
    private var enterLabel = "⏎"

    init {
        setBackgroundColor(palette.background)
        val column = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
        column.addView(bar, LinearParams(LinearParams.MATCH_PARENT, LinearParams.WRAP_CONTENT))
        column.addView(keys, LinearParams(LinearParams.MATCH_PARENT, LinearParams.WRAP_CONTENT))
        addView(column, FrameParams(FrameParams.MATCH_PARENT, FrameParams.WRAP_CONTENT))
        addView(preview, FrameParams(0, 0))
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
        shift = Shift.OFF
        layoutKeys()
    }

    private fun layoutKeys() {
        keys.removeAllViews()
        if (symbols) {
            row("1234567890".map { symbolKey(it.toString()) })
            row("-/:;()៛&@\"".map { symbolKey(it.toString()) })
            row(".,?!'%+=".map { symbolKey(it.toString()) } + deleteKey())
        } else {
            row("qwertyuiop".zip("1234567890").map { (letter, digit) -> letterKey(letter, digit) })
            row(listOf(spacer(0.5f)) + "asdfghjkl".map { letterKey(it) } + spacer(0.5f))
            row(listOf(shiftKey()) + "zxcvbnm".map { letterKey(it) } + deleteKey())
        }
        val bottom = mutableListOf(
            key(if (symbols) "ABC" else "123", 1.5f, special = true) {
                symbols = !symbols
                layoutKeys()
            },
        )
        if (showSwitchKey) bottom += globeKey()
        bottom += key(context.getString(R.string.space), 4.5f) { listener.onSpace() }
        bottom += key("។", 1f, hint = "៕", preview = true) { listener.onSymbol("។") }
            .holding("៕") { listener.onSymbol("៕") }
        bottom += key(enterLabel, 1.5f, special = true, label = R.string.enter) {
            listener.onEnter()
        }
        row(bottom)
    }

    private fun row(views: List<View>) {
        val row = LinearLayout(context).apply { orientation = LinearLayout.HORIZONTAL }
        for (view in views) row.addView(view)
        keys.addView(row, LinearParams(LinearParams.MATCH_PARENT, keyHeight))
    }

    private fun letterKey(letter: Char, digit: Char? = null): View {
        val shown = (if (shift == Shift.OFF) letter else letter.uppercaseChar()).toString()
        val view = key(shown, 1f, hint = digit?.toString(), preview = true) {
            listener.onLetter(shown)
            if (shift == Shift.ONCE) {
                shift = Shift.OFF
                layoutKeys()
            }
        }
        if (digit == null) return view
        return view.holding(digit.toString()) { listener.onSymbol(digit.toString()) }
    }

    private fun symbolKey(symbol: String) =
        key(symbol, 1f, preview = true) { listener.onSymbol(symbol) }

    private fun shiftKey(): View {
        val label = when (shift) {
            Shift.OFF -> "⇧"
            Shift.ONCE -> "⬆"
            Shift.LOCKED -> "⇪"
        }
        return key(label, 1.5f, special = true, label = R.string.shift) {
            val now = SystemClock.uptimeMillis()
            shift = when {
                shift == Shift.ONCE && now - lastShiftTap < DOUBLE_TAP -> Shift.LOCKED
                shift == Shift.OFF -> Shift.ONCE
                else -> Shift.OFF
            }
            lastShiftTap = now
            layoutKeys()
        }
    }

    private fun globeKey(): View {
        val icon = ImageView(context).apply {
            setImageResource(R.drawable.ic_language)
            setColorFilter(palette.text)
            scaleType = ImageView.ScaleType.CENTER
        }
        return cell(icon, 1f, special = true).apply {
            contentDescription = context.getString(R.string.switch_keyboard)
            setOnClickListener {
                it.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                listener.onSwitchKeyboard()
            }
            setOnLongClickListener {
                listener.onChooseKeyboard()
                true
            }
        }
    }

    /** Delete repeats while held, after a short wait. */
    @SuppressLint("ClickableViewAccessibility")
    private fun deleteKey(): View {
        val view = key("⌫", 1.5f, special = true, label = R.string.delete) { listener.onDelete() }
        val tick = object : Runnable {
            override fun run() {
                listener.onDelete()
                handler.postDelayed(this, REPEAT_INTERVAL)
            }
        }
        view.setOnTouchListener { v, event ->
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    v.isPressed = true
                    v.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    listener.onDelete()
                    handler.postDelayed(tick, REPEAT_DELAY)
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                    v.isPressed = false
                    handler.removeCallbacks(tick)
                }
            }
            true
        }
        return view
    }

    private fun spacer(weight: Float) = View(context).apply {
        layoutParams = LinearParams(0, LinearParams.MATCH_PARENT, weight)
    }

    /** A key showing [text], with an optional [hint] in its corner. */
    @SuppressLint("ClickableViewAccessibility")
    private fun key(
        text: String,
        weight: Float,
        special: Boolean = false,
        label: Int? = null,
        hint: String? = null,
        preview: Boolean = false,
        onTap: () -> Unit,
    ): View {
        val main = TextView(context).apply {
            this.text = text
            gravity = Gravity.CENTER
            textSize = if (text.length > 1) 16f else 22f
            setTextColor(palette.text)
        }
        val cell = cell(main, weight, special)
        if (hint != null) {
            val corner = TextView(context).apply {
                this.text = hint
                textSize = 10f
                setTextColor(palette.muted)
                setPadding(0, palette.dp(2f), palette.dp(5f), 0)
            }
            val at = Gravity.TOP or Gravity.END
            cell.addView(corner, FrameParams(FrameParams.WRAP_CONTENT, FrameParams.WRAP_CONTENT, at))
        }
        cell.contentDescription = label?.let(context::getString) ?: text
        cell.setOnClickListener {
            it.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
            onTap()
        }
        if (preview) {
            cell.setOnTouchListener { v, event ->
                when (event.actionMasked) {
                    MotionEvent.ACTION_DOWN -> showPreview(v, text)
                    MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL ->
                        handler.postDelayed(hidePreview, PREVIEW_LINGER)
                }
                false // the key still handles the tap and the long press
            }
        }
        return cell
    }

    /** Holding the key types [alternate] instead, shown in the preview while held. */
    private fun View.holding(alternate: String, onHold: () -> Unit): View = apply {
        setOnLongClickListener {
            it.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
            preview.text = alternate
            onHold()
            true
        }
    }

    private fun cell(content: View, weight: Float, special: Boolean = false) =
        FrameLayout(context).apply {
            background = palette.keyBackground(if (special) palette.special else palette.key)
            isSoundEffectsEnabled = true
            addView(content, FrameParams(FrameParams.MATCH_PARENT, FrameParams.MATCH_PARENT))
            layoutParams = LinearParams(0, LinearParams.MATCH_PARENT, weight).apply {
                setMargins(gap, gap, gap, gap)
            }
        }

    /** Show [text] enlarged over the top of [key], reaching above it. */
    private fun showPreview(key: View, text: String) {
        handler.removeCallbacks(hidePreview)
        val keyAt = IntArray(2).also(key::getLocationInWindow)
        val selfAt = IntArray(2).also(::getLocationInWindow)
        val width = key.width + 2 * gap
        val height = (key.height * PREVIEW_SCALE).toInt()
        val top = keyAt[1] - selfAt[1] + key.height * PREVIEW_OVERLAP - height
        preview.text = text
        preview.layoutParams = FrameParams(width, height)
        preview.x = (keyAt[0] - selfAt[0] - gap).toFloat()
        preview.y = top.coerceAtLeast(0f)
        preview.visibility = VISIBLE
    }

    override fun onDetachedFromWindow() {
        handler.removeCallbacksAndMessages(null)
        super.onDetachedFromWindow()
    }

    private companion object {
        const val REPEAT_DELAY = 400L
        const val REPEAT_INTERVAL = 50L
        const val DOUBLE_TAP = 300L
        const val PREVIEW_LINGER = 70L
        const val PREVIEW_SCALE = 1.3f
        const val PREVIEW_OVERLAP = 0.55f // how much of the key the preview covers
    }
}
