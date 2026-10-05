package io.github.pheasakhmer.keyboard

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import io.github.pheasakhmer.keyboard.core.Keyboard
import io.github.pheasakhmer.keyboard.core.KeyboardException
import java.io.File
import java.util.concurrent.Executors

/** [Core] backed by the Rust core. */
class NativeCore(val keyboard: Keyboard) : Core {
    // The core's offsets count characters; typed text is Latin, so they are also
    // Kotlin string indices.
    override fun suggest(context: String, typed: String, count: Int): List<Reading> =
        keyboard.suggest(context, typed, count.toUInt()).map {
            Reading(it.text, it.start.toInt(), it.end.toInt(), it.source in LATIN_SOURCES)
        }

    override fun convert(text: String): String = keyboard.convert(text)

    override fun learn(typed: String, word: String) {
        try {
            keyboard.learn(typed, word)
        } catch (error: KeyboardException) {
            Log.w(TAG, "could not save a learned word", error)
        }
    }

    private companion object {
        val LATIN_SOURCES = setOf("english", "typed")
    }
}

/**
 * Loads the core once per process, off the main thread: the data file is about 10 MB.
 * The keyboard and the setup screen share it, so clearing learned words in one is seen
 * by the other.
 */
object CoreLoader {
    private const val DATA_ASSET = "khmer.kbd"
    private const val LEARNED_FILE = "learned.tsv"

    private val executor = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())

    @Volatile
    private var core: NativeCore? = null

    /** Calls [ready] on the main thread with the core, loading it first if needed. */
    fun load(context: Context, ready: (NativeCore) -> Unit) {
        core?.let { return ready(it) }
        val app = context.applicationContext
        executor.execute {
            val loaded = core ?: open(app).also { core = it }
            main.post { loaded?.let(ready) }
        }
    }

    private fun open(context: Context): NativeCore? = try {
        val data = context.assets.open(DATA_ASSET).use { it.readBytes() }
        val learned = File(context.filesDir, LEARNED_FILE).path
        NativeCore(Keyboard.fromBytes(data, learned))
    } catch (error: KeyboardException) {
        Log.e(TAG, "could not load the keyboard data", error)
        null
    }
}

internal const val TAG = "KhmerKeyboard"
