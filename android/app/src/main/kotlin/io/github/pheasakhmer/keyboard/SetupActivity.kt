package io.github.pheasakhmer.keyboard

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import io.github.pheasakhmer.keyboard.core.KeyboardException

/**
 * The app's own screen: how to turn the keyboard on, a field to try it, and a button to
 * forget learned words.
 */
class SetupActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val palette = Palette(this)
        val pad = palette.dp(20f)
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad, pad, pad)
        }
        fun text(id: Int, size: Float = 16f) = TextView(this).apply {
            setText(id)
            textSize = size
            setPadding(0, palette.dp(8f), 0, palette.dp(8f))
        }
        fun button(id: Int, onTap: () -> Unit) = Button(this).apply {
            setText(id)
            setOnClickListener { onTap() }
        }

        column.addView(text(R.string.app_name, 24f))
        column.addView(text(R.string.setup_intro))
        column.addView(text(R.string.setup_step_enable))
        column.addView(button(R.string.open_settings) {
            startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS))
        })
        column.addView(text(R.string.setup_step_switch))
        column.addView(button(R.string.choose_keyboard) {
            getSystemService(InputMethodManager::class.java).showInputMethodPicker()
        })
        column.addView(text(R.string.setup_step_try))
        column.addView(EditText(this).apply {
            setHint(R.string.try_hint)
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
            minLines = 3
        })
        column.addView(text(R.string.privacy))
        column.addView(button(R.string.forget_learned) { forgetLearned() })
        // Keep the content clear of the status and navigation bars, which Android 15 and
        // later draw over the app.
        setContentView(ScrollView(this).apply {
            fitsSystemWindows = true
            addView(column)
        })
    }

    private fun forgetLearned() {
        CoreLoader.load(this) { core ->
            val message = try {
                core.keyboard.forget()
                R.string.forgot_learned
            } catch (_: KeyboardException) {
                R.string.forget_failed
            }
            Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
        }
    }
}
