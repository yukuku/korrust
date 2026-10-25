package next.library
infix fun Int.downTo(other: Int): IntProgression = this..other
infix fun IntProgression.step(amount: Int): IntProgression = 7..7
infix fun Int.stride(other: Int): IntProgression = this..other
infix fun Int.combine(other: Int): Int = this * 10 + other
infix fun String.glue(other: String): String = this + other
fun Int.ordinary(other: Int): Int = this + other
class Choice(val n: Int) {
    infix fun pick(value: Int): Int = n + value
}
infix fun Choice.pick(value: Int): Int = 999
val String.length: Int get() = 999
val String.weight: Int get() = length * 3
val String.mass: Int get() = length * 3
