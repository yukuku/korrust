import regions.dependency.*

fun normal(log: IntArray): Int = bracket(log, 1) { 11 }
fun thrown(log: IntArray): Int = recover(log) { throw IllegalArgumentException("body") }
fun nonlocal(log: IntArray): Long {
    bracket<Long>(log, 1) { return 10000000001L }
    return 0L
}
fun nestedReturn(log: IntArray): Int {
    nested(log) { return 23 }
    return 0
}
fun sourceFinally(log: IntArray): Int {
    try {
        nested(log) { return 31 }
        return 0
    } finally {
        log[0] = log[0] * 10 + 1
    }
}
fun finalizerThrow(log: IntArray): Int {
    try {
        exploding(log) { return 41 }
    } catch (error: IllegalStateException) {
        return log[0]
    }
    return 0
}
fun finalizerLambda(log: IntArray): Int {
    finish(log, { return 51 }, { log[0] = log[0] * 10 + 2 })
    return 0
}
fun overrideReturn(log: IntArray): Int {
    finish(log, { return 51 }, { return 61 })
    return 0
}
fun localReturn(log: IntArray): Int = bracket<Int>(log, 1) point@ { return@point 71 }
fun reified(value: Any?, log: IntArray): Boolean = classify<String?>(value, log)
