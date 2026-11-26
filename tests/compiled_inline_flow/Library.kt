package flow
inline fun changed(a: IntArray, i: Int, block: (Int) -> Int): Int {
    val old = a[i]++
    return block(old) + a[i]
}
inline fun changedLong(a: LongArray, i: Int, block: (Long) -> Long): Long {
    val old = a[i]++
    return block(old) + a[i]
}
inline fun doubleSum(a: DoubleArray, block: (Double) -> Double): Double {
    var result = 0.0
    for (value in a) result += block(value)
    return result
}
inline fun numeric(value: Double, block: (Double) -> Double): Double {
    val a = -value / 2.0
    return block(a) + a.toFloat().toDouble() + value.toLong().toDouble()
}
inline fun dense(value: Int, block: (Int) -> Int): Int = when (value) {
    0 -> block(10)
    1 -> block(20)
    2 -> block(30)
    else -> block(-1)
}
inline fun sparse(value: Int, block: (Int) -> Int): Int = when (value) {
    -10000 -> block(2)
    0 -> block(3)
    10000 -> block(5)
    else -> block(7)
}
inline fun materialized(value: Double, block: (DoubleArray) -> Double): Double {
    val values = doubleArrayOf(value, -value)
    return block(values)
}
inline fun ranked(value: Double, block: (Int) -> Int): Int = block(if (value > 1.0) 2 else if (value < 0.0) -2 else 0)

inline fun applyDefault(value: Int, block: (Int) -> Int = { it }): Int = block(value)
