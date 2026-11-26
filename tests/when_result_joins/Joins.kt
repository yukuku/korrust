package whenjoins

open class Base(val code: Int)
class Child(code: Int) : Base(code)

fun nullable(value: Int): Base? = when (value) {
    0 -> Child(10)
    1 -> Base(20)
    else -> null
}
fun nullFirst(value: Int): Base? = when (value) {
    0 -> null
    1 -> Child(30)
    else -> Base(40)
}
fun nullBranch(value: Boolean): String? = when (value) {
    true -> null
    else -> "value"
}
fun exhaustive(value: Boolean): Base? = when (value) {
    true -> Child(50)
    false -> null
}
fun throwing(value: Boolean): String {
    val result = when (value) {
        true -> "normal"
        else -> throw IllegalArgumentException("abrupt")
    }
    return result
}
fun returning(value: Boolean): String {
    val result = when (value) {
        true -> "normal"
        else -> return "returned"
    }
    return result
}
class Probe {
    var hits: Int = 0
    fun hit(code: Int): Base { hits = hits + 1; return Base(code) }
    fun select(value: Int): Base? = when (value) {
        0 -> hit(60)
        1 -> hit(70)
        else -> null
    }
}

fun boxed(value: Boolean): Int? = when (value) { true -> 17; else -> null }
fun wide(value: Boolean): Long? = when (value) { true -> 4294967298L; else -> null }
