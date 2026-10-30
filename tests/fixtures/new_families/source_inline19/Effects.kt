package lambdas
inline fun apply(x: Int, block: (Int) -> Int): Int = block(x) + block(x+1)
inline fun runCross(crossinline block: () -> Int): Int = block()
inline fun returnNo(noinline block: () -> Int): () -> Int = block
fun direct(): Int = apply(5) { it * 2 }
fun nonlocal(): Int { apply(5) { return it }; return 99 }
fun cross(): Int = runCross { 23 }
fun no(): Int { val f = returnNo { 8 }; return f() }
fun cleanup(): Int { try { apply(5) { return it } } finally { effects = effects + 1 }; return 99 }
var effects: Int = 0
fun captured(): Int { var n = 3; val value = apply(2) { n = n + it; n }; return value * 100 + n }
fun cleanupLambda(): Int { effects = 0; try { apply(1) { try { return it } finally { effects = effects + 10 } } } finally { effects = effects + 100 }; return 99 }
inline fun <R> genericApply(value: Int, block: (Int) -> R): R = block(value)
fun generic(): String = genericApply(3) { "v" + it }
fun genericReturn(): Int { genericApply<Int>(5) { return it }; return 9 }
private inline fun privateTwice(value: Int): Int = value * 2
fun privateCall(): Int = privateTwice(7)
private fun helperOverload(value: String): Int = 0
fun helperOverload(value: Int): Int = value + 1
inline fun publicOverload(value: Int): Int = helperOverload(value)
fun overloaded(): Int = publicOverload(7)
inline fun <R> genericNoArg(block: () -> R): R = block()
fun conditionalNonlocal(early: Boolean): Int {
    val result = genericNoArg { if (early) return 7; 11 }
    return result + 1
}
fun conditionalFinally(early: Boolean): Int {
    effects = 0
    try {
        val result = genericNoArg { if (early) return 7; 11 }
        return result + 1
    } finally { effects = effects + 1 }
}
