package operators

object Events {
    var trace: String = ""
    fun reset() { trace = "" }
    fun mark(text: String) { trace += text }
}

open class Measure(val value: Int) {
    open operator fun unaryPlus(): Measure { Events.mark("+"); return Measure(value + 10) }
    open operator fun unaryMinus(): Measure { Events.mark("-"); return Measure(-value) }
    operator fun not(): Int { Events.mark("!"); return value + 100 }
    open operator fun compareTo(other: Measure): Int {
        Events.mark("c")
        return if (value < other.value) Int.MIN_VALUE else if (value > other.value) Int.MAX_VALUE else 0
    }
}
class Shifted(value: Int): Measure(value) {
    override fun unaryMinus(): Measure { Events.mark("d"); return Measure(value + 20) }
    override fun compareTo(other: Measure): Int { Events.mark("v"); return 0 }
}
class Plain(val value: Int)
operator fun Plain.unaryPlus(): Plain { Events.mark("p"); return Plain(value + 1) }
operator fun Plain.unaryMinus(): Plain { Events.mark("m"); return Plain(-value) }
operator fun Plain.not(): String { Events.mark("n"); return "custom" }
operator fun Plain.compareTo(other: Plain): Int { Events.mark("e"); return value - other.value }
operator fun Measure.unaryPlus(): Measure = Measure(999)
operator fun Measure.compareTo(other: Measure): Int = 0
fun left(value: Int): Measure { Events.mark("L"); return Measure(value) }
fun right(value: Int): Measure { Events.mark("R"); return Measure(value) }
fun sourceUnary(): Int = (+(-left(3))).value + !Measure(4)
fun extensionUnary(): Int = (+(-Plain(5))).value
fun extensionNot(): String = !Plain(1)
fun precedence(): Int = -Measure(3).value + (!Measure(1)) * 2
fun compare(a: Int, b: Int, mode: Int): Boolean = when(mode) {
    0 -> left(a) < right(b)
    1 -> left(a) <= right(b)
    2 -> left(a) > right(b)
    else -> left(a) >= right(b)
}
fun extensionCompare(a: Int, b: Int): Boolean = Plain(a) < Plain(b)
fun dispatch(value: Measure): Int = (-value).value
fun dispatchCompare(value: Measure): Boolean = value >= Measure(900)
fun fails(): Measure { Events.mark("F"); throw IllegalStateException("operand") }
fun unaryFailure(): Int = !fails()
fun compareFailure(): Boolean = left(1) < fails()
fun directDispatch(): Int = (-Shifted(4)).value
fun directCompare(): Boolean = Shifted(4) >= Measure(900)
open class Origin { open operator fun unaryMinus(): Int = 1 }
open class Middle: Origin() { override fun unaryMinus(): Int = 2 }
class Descendant: Middle() { override fun unaryMinus(): Int = 3 }
fun grandparentInherited(): Int = -Descendant()
interface BaseFlag { operator fun not(): Int }
interface DerivedFlag: BaseFlag { override fun not(): Int }
class Switch: DerivedFlag { override fun not() = 7 }
fun sourceInterface(value: DerivedFlag): Int = !value
fun sourceImplementation(): Int = !Switch()
