package compatibility.visibility

private val fileSecret: Int = 11

private fun privateOffset(value: Int): Int = value + fileSecret

internal fun moduleOnly(value: Int): Int = privateOffset(value)

object InternalObject {
    internal fun decorate(value: Int): Int = value + 7

    fun publicCall(value: Int): Int = decorate(value)

    fun sameFilePrivate(value: Int): Int = privateOffset(value) + fileSecret
}

fun visible(value: Int): Int = InternalObject.decorate(moduleOnly(value))
