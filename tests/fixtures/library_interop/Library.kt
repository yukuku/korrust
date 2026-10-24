package interop.lib

import interop.dep.JavaApi

fun lifted(value: Int): Int = value + 13
fun <T> identity(value: T): T = value
fun <T : CharSequence> bounded(value: T): Int = value.length
fun String.scaled(factor: Int): Int = length * factor
val libraryNumber: Int get() = 41
var libraryMutable: Int = 4
fun defaults(a: Int = JavaApi.mark("a", 2), b: Int = JavaApi.mark("b", 3)): Int = a * 10 + b
fun applyValue(value: Int, block: (Int) -> Int): Int = block(value)
inline fun actualInline(value: Int, block: (Int) -> Int): Int = block(value) + 7
inline fun gate(block: () -> Int): Int = block() + 9
class Counter(val base: Int) {
    fun add(value: Int): Int = base + value
}
interface ValueSource {
    fun value(): Int
}
open class BaseSource {
    open fun value(): Int = 6
}
class DerivedSource : BaseSource(), ValueSource {
    override fun value(): Int = 12
}

fun counterReceiver(): Counter { JavaApi.mark("R", 0); return Counter(10) }
inline fun noinlineGate(noinline block: () -> Int): Int = block()
inline fun crossinlineGate(crossinline block: () -> Int): Int = block()
inline fun allocationGate(block: () -> Int): Int {
    val token = Any()
    return block() + token.hashCode()
}
inline fun handlerGate(block: () -> Int): Int {
    try { return block() } catch (failure: IllegalStateException) { return -2 }
}
inline fun twice(block: () -> Int): Int = block() + block()
inline fun branch(flag: Boolean, block: (Int) -> Int): Int {
    var i = 0
    var result = 0
    while (i < 3) {
        if (flag && i == 1) result += block(i) else result += i
        i++
    }
    return result
}
inline fun inlineSimple(value: Int): Int = value * 2
