package batch.sample

val seed: Int = 7
fun forward() = 5
fun odd(value: Int): Boolean = if (value == 0) false else even(value - 1)
class Right(val left: Left?)
fun choose(value: Int): Int = value + 1
fun weighted(value: Int, scale: Int = 3): Int = value * scale
fun <T> identity(value: T): T = value
class Holder(val value: Int, val bonus: Int = 2) {
    companion object {
        val DEFAULT = Holder(value = seed)
        fun make(value: Int): Holder = Holder(value)
    }
}
internal fun moduleOnly(): Int = 17
