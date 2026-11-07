package anonymouscompiled
import anonymouslibrary.Pair
fun make(base: Int): Pair = object : Pair {
    override fun value(): Int = base
    override fun shift(delta: Int): Int = value() + delta
}
