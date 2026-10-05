package io.github.pheasakhmer.keyboard

import android.inputmethodservice.InputMethodService
import android.os.Build
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager

/**
 * The keyboard. Latin letters are composed in the field (underlined) while the suggestion
 * bar shows Khmer readings; space commits the best one, a tap commits any other.
 *
 * In password, email, web address and number fields, keys type exactly what they show:
 * nothing is converted, suggested or learned there.
 */
class KhmerKeyboardService : InputMethodService(), KeyListener {
    private val composer = Composer()
    private var view: KeyboardView? = null
    private var literal = false

    override fun onCreate() {
        super.onCreate()
        CoreLoader.load(this) { composer.core = it }
    }

    override fun onCreateInputView(): View = KeyboardView(this, this).also { view = it }

    override fun onStartInputView(info: EditorInfo, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        composer.reset()
        val inputClass = info.inputType and InputType.TYPE_MASK_CLASS
        val variation = info.inputType and InputType.TYPE_MASK_VARIATION
        val numeric = inputClass == InputType.TYPE_CLASS_NUMBER ||
            inputClass == InputType.TYPE_CLASS_PHONE ||
            inputClass == InputType.TYPE_CLASS_DATETIME
        literal = numeric || inputClass == InputType.TYPE_CLASS_TEXT && variation in LITERAL_VARIATIONS
        view?.reset(enterLabel(info), showSwitchKey(), startWithSymbols = numeric)
        showBar()
    }

    override fun onFinishInput() {
        composer.reset()
        super.onFinishInput()
    }

    override fun onUpdateSelection(
        oldSelStart: Int,
        oldSelEnd: Int,
        newSelStart: Int,
        newSelEnd: Int,
        candidatesStart: Int,
        candidatesEnd: Int,
    ) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd)
        // The cursor left the word being typed (the user tapped elsewhere): leave it as typed.
        val moved = newSelStart != candidatesEnd || newSelEnd != candidatesEnd
        if (composer.typed.isNotEmpty() && moved) {
            composer.reset()
            currentInputConnection?.finishComposingText()
            showBar()
        }
    }

    override fun onLetter(letter: String) {
        val ic = currentInputConnection ?: return
        if (literal) {
            ic.commitText(letter, 1)
            return
        }
        composer.type(letter, context())
        ic.setComposingText(composer.typed, 1)
        showBar()
    }

    override fun onSymbol(symbol: String) {
        commit(composer.accept())
        currentInputConnection?.commitText(symbol, 1)
    }

    override fun onSpace() {
        val candidate = composer.accept()
        if (candidate == null) currentInputConnection?.commitText(" ", 1) else commit(candidate)
    }

    override fun onDelete() {
        val ic = currentInputConnection ?: return
        if (composer.typed.isEmpty()) {
            ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DEL))
            ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DEL))
            return
        }
        composer.deleteLast(context())
        ic.setComposingText(composer.typed, 1)
        if (composer.typed.isEmpty()) ic.finishComposingText()
        showBar()
    }

    override fun onEnter() {
        commit(composer.accept())
        val ic = currentInputConnection ?: return
        val info = currentInputEditorInfo
        val action = info.imeOptions and EditorInfo.IME_MASK_ACTION
        val multiLine = info.inputType and InputType.TYPE_TEXT_FLAG_MULTI_LINE != 0
        val noAction = info.imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION != 0
        if (!multiLine && !noAction && action != EditorInfo.IME_ACTION_NONE &&
            action != EditorInfo.IME_ACTION_UNSPECIFIED
        ) {
            ic.performEditorAction(action)
        } else {
            ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER))
            ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER))
        }
    }

    override fun onSwitchKeyboard() {
        commit(composer.accept())
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            switchToNextInputMethod(false)
        } else {
            @Suppress("DEPRECATION")
            getSystemService(InputMethodManager::class.java)
                .switchToNextInputMethod(window.window?.attributes?.token, false)
        }
    }

    override fun onPick(candidate: Candidate) = commit(composer.pick(candidate))

    override fun onKeepTyped() = commit(composer.keepTyped())

    /** Replace the composing text with [candidate]. Latin keeps a space after it. */
    private fun commit(candidate: Candidate?) {
        val ic = currentInputConnection ?: return
        if (candidate != null) {
            ic.commitText(if (candidate.isLatin) candidate.text + " " else candidate.text, 1)
        }
        showBar()
    }

    /** The text before the word being typed, whose last Khmer word is the context. */
    private fun context(): String {
        val typed = composer.typed
        val before = currentInputConnection?.getTextBeforeCursor(CONTEXT_LENGTH + typed.length, 0)
            ?.toString().orEmpty()
        return if (typed.isNotEmpty() && before.endsWith(typed)) before.dropLast(typed.length) else before
    }

    private fun showBar() {
        view?.bar?.show(composer.bar)
    }

    private fun showSwitchKey(): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.P || shouldOfferSwitchingToNextInputMethod()

    private fun enterLabel(info: EditorInfo): String =
        when (info.imeOptions and EditorInfo.IME_MASK_ACTION) {
            EditorInfo.IME_ACTION_SEARCH -> "🔍"
            EditorInfo.IME_ACTION_SEND -> "➤"
            EditorInfo.IME_ACTION_GO, EditorInfo.IME_ACTION_NEXT -> "→"
            else -> "⏎"
        }

    private companion object {
        const val CONTEXT_LENGTH = 64
        val LITERAL_VARIATIONS = setOf(
            InputType.TYPE_TEXT_VARIATION_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
            InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS,
            InputType.TYPE_TEXT_VARIATION_URI,
        )
    }
}
