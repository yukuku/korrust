package nonfinal.library

var order: Int = 0

fun record(value: Int): Int {
    order = order * 10 + value
    return value
}

fun pack(prefix: Int = 1, vararg values: Int, suffix: Int = 2, block: (Int) -> Int = { it }): Int {
    var total = prefix * 100 + suffix
    for (value in values) total += value
    return block(total)
}

fun <T> tail(vararg values: T, last: T): T = if (values.isEmpty()) last else values[0]

class Collector {
    fun pack(vararg values: Int, suffix: Int): Int = values.sum() + suffix
}

class Collected(vararg values: Int, suffix: Int) {
    val total: Int = values.sum() + suffix
}

fun Int.collect(vararg values: Int, suffix: Int): Int = this + values.sum() + suffix

fun ownDefault(vararg values: Int = intArrayOf(9), suffix: Int = 2): Int = values.sum() + suffix

fun choose(value: Int, block: () -> Int): Int = value + block()
fun choose(vararg values: Int, block: () -> Int): Int = values.sum() + block() + 1000
