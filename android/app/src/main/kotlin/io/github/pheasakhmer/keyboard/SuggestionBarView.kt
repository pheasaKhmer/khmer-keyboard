package io.github.pheasakhmer.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Typeface
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView

/**
 * The suggestion bar, laid out like the iPhone's: the typed text in quotes on the left
 * (tap to keep it in Latin letters), the best candidate in the middle (what space
 * commits), the next one on the right. Khmer often has more readings than fit, so an
 * arrow opens a row with the rest.
 */
@SuppressLint("ViewConstructor")
class SuggestionBarView(
    context: Context,
    private val palette: Palette,
    private val onPick: (Candidate) -> Unit,
    private val onKeepTyped: () -> Unit,
) : LinearLayout(context) {
    private val leftSlot = slot()
    private val middleSlot = slot().apply { setTypeface(typeface, Typeface.BOLD) }
    private val rightSlot = slot()
    private val arrow = TextView(context).apply {
        text = "⌄"
        gravity = Gravity.CENTER
        textSize = 20f
        setTextColor(palette.muted)
        contentDescription = context.getString(R.string.more_suggestions)
        setOnClickListener { expanded = !expanded; render() }
    }
    private val moreRow = LinearLayout(context).apply { orientation = HORIZONTAL }
    private val more = HorizontalScrollView(context).apply {
        isHorizontalScrollBarEnabled = false
        addView(moreRow)
        visibility = GONE
    }

    private var bar: Bar = Bar.EMPTY
    private var expanded = false

    init {
        orientation = VERTICAL
        val row = LinearLayout(context).apply {
            orientation = HORIZONTAL
            minimumHeight = palette.dp(44f)
            addView(leftSlot, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
            addView(divider())
            addView(middleSlot, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
            addView(divider())
            addView(rightSlot, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
            addView(arrow, LinearLayout.LayoutParams(palette.dp(40f), LinearLayout.LayoutParams.MATCH_PARENT))
        }
        addView(row, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, palette.dp(44f)))
        addView(more, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, palette.dp(44f)))
        render()
    }

    fun show(bar: Bar) {
        this.bar = bar
        if (bar.typed.isEmpty()) expanded = false
        render()
    }

    private fun render() {
        val best = bar.best
        // The typed text has its own slot unless it already is the best candidate.
        val typedIsBest = best != null && best.text == bar.typed
        val others = bar.others.filter { typedIsBest || it.text != bar.typed }
        val (leftCandidate, rest) = if (typedIsBest) others.firstOrNull() to others.drop(1) else null to others

        when {
            bar.typed.isEmpty() -> fill(leftSlot, null)
            leftCandidate != null -> fill(leftSlot, leftCandidate)
            else -> {
                leftSlot.text = "“${bar.typed}”"
                leftSlot.setTextColor(palette.muted)
                leftSlot.setOnClickListener { onKeepTyped() }
            }
        }
        fill(middleSlot, best)
        fill(rightSlot, rest.firstOrNull())

        val hidden = rest.drop(1)
        arrow.visibility = if (hidden.isEmpty()) INVISIBLE else VISIBLE
        arrow.text = if (expanded) "⌃" else "⌄"
        moreRow.removeAllViews()
        for (candidate in hidden) {
            moreRow.addView(
                slot().also { fill(it, candidate) },
                LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.MATCH_PARENT),
            )
        }
        more.visibility = if (expanded && hidden.isNotEmpty()) VISIBLE else GONE
    }

    private fun fill(view: TextView, candidate: Candidate?) {
        view.text = candidate?.text.orEmpty()
        view.setTextColor(palette.text)
        view.setOnClickListener(candidate?.let { c -> View.OnClickListener { onPick(c) } })
        view.isClickable = candidate != null
    }

    private fun slot() = TextView(context).apply {
        gravity = Gravity.CENTER
        textSize = 18f
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.MIDDLE
        setPadding(palette.dp(8f), 0, palette.dp(8f), 0)
        setTextColor(palette.text)
    }

    private fun divider() = View(context).apply {
        setBackgroundColor(palette.divider)
        layoutParams = LinearLayout.LayoutParams(palette.dp(1f), palette.dp(24f)).apply {
            gravity = Gravity.CENTER_VERTICAL
        }
    }
}
