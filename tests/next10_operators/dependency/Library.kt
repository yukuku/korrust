package unusual

class Dial(val value: Int) {
    operator fun unaryPlus(): Dial = Dial(value + 8)
    operator fun unaryMinus(): Dial = Dial(-value)
    operator fun not(): Long = value.toLong() + 500L
    operator fun compareTo(other: Dial): Int = if(value < other.value) Int.MIN_VALUE else if(value > other.value) Int.MAX_VALUE else 0
}
class External(val value: Int)
operator fun External.unaryPlus(): External = External(value + 2)
operator fun External.unaryMinus(): External = External(-value)
operator fun External.not(): String { Trace.mark("N"); return "external" }
operator fun External.compareTo(other: External): Int { Trace.mark("C"); return value - other.value }
interface Ranking { operator fun compareTo(other: Ranking): Int }
class Ranked(val value: Int): Ranking { override fun compareTo(other: Ranking): Int = value }
open class Parent(val value: Int) {
    open operator fun unaryMinus(): Parent = Parent(-value)
    open operator fun compareTo(other: Parent): Int = value - other.value
}
class Child(value: Int): Parent(value) {
    override fun unaryMinus(): Parent = Parent(value + 30)
    override fun compareTo(other: Parent): Int = 0
}
object Trace {
    var trace: String = ""
    fun reset() { trace = "" }
    fun mark(text: String) { trace += text }
}
fun makeLeft(value: Int): External { Trace.mark("L"); return External(value) }
fun makeRight(value: Int): External { Trace.mark("R"); return External(value) }
open class Up { open operator fun unaryMinus(): Up = Up() }
class Down: Up() { override fun unaryMinus(): Down = this }
