package delegated
import delegated.Events
interface Contract<T> { fun echo(value:T):T; var value:T; val label:String }
class Forward<T>(val impl: Contract<T>): Contract<T> by impl
class IntForward(impl: Contract<Int>): Contract<Int> by impl {
    override fun echo(value:Int):Int=value+1
}
class RefForward(impl: Contract<String>): Contract<String> by impl
interface A { fun one(value:Int):Int }
interface B { var count:Int }
class PairForward(a:A,b:B):A by a,B by b
class PlainIntForward(impl: Contract<Int>): Contract<Int> by impl
interface Counter {
    var count: Int
    val name: String
    fun read(): Int
    fun twice(value: Long): Long
}
fun pickCounter(value: Counter): Counter = Events.pick(value, 1) as Counter
class Saved(var current: Counter) : Counter by pickCounter(current) {
    val initialized: Int = Events.mark(3)
    init { Events.mark(4) }
}
fun pickA(value: A): A = Events.pick(value, 1) as A
fun pickB(value: B): B = Events.pick(value, 2) as B
class Ordered(a: A, b: B) : A by pickA(a), B by pickB(b) {
    val initialized: Int = Events.mark(3)
    init { Events.mark(4) }
}
class OverrideCounter(delegate: Counter): Counter by delegate {
    override var count: Int = 11
    override fun read(): Int = 99
}
interface Parent<T> { fun echo(value: T): T }
interface Child<T>: Parent<T>
class Inherited<T>(delegate: Parent<T>) : Child<T>, Parent<T> by delegate
fun scalar(value: PlainIntForward): Int = value.value
fun stringValue(value: RefForward): String = value.value
class Collision(val `$$delegate_0`: Counter) : Counter by `$$delegate_0`
interface Left { fun value(): Int }
interface Right { fun value(): Int }
class Shared(a: Left, b: Right) : Left by a, Right by b { override fun value(): Int = 77 }
class Caught(delegate: Counter): Counter by try {
    if (Events.mark(5) == 5) throw IllegalStateException("delegate")
    delegate
} catch (error: IllegalStateException) {
    delegate
}
class JavaForward(delegate: JavaText) : JavaText by delegate
open class OpenForward(delegate: Counter): Counter by delegate
class Derived(delegate: Counter): OpenForward(delegate)
fun platformConsumer(value: JavaForward): String? = value.echo(null)
class LongForward(delegate: Contract<Long>): Contract<Long> by delegate
class FloatForward(delegate: Contract<Float>): Contract<Float> by delegate
class DoubleForward(delegate: Contract<Double>): Contract<Double> by delegate
class ByteForward(delegate: Contract<Byte>): Contract<Byte> by delegate
class BooleanForward(delegate: Contract<Boolean>): Contract<Boolean> by delegate
class CharForward(delegate: Contract<Char>): Contract<Char> by delegate
class NullableForward(delegate: Contract<Int?>): Contract<Int?> by delegate
interface DiamondRoot { fun shared(): Int }
interface DiamondLeft : DiamondRoot
interface DiamondRight : DiamondRoot
interface Diamond : DiamondLeft, DiamondRight
class DiamondForward(delegate: Diamond) : Diamond by delegate
