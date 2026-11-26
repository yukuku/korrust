package constructorvarargs

class Strings(val prefix: String, vararg val values: String)
class Ints(vararg val values: Int)
class Wide(vararg val values: Long)
class Nullable(vararg val values: String?)
class Generic<T>(vararg val values: T)
class Mutable(vararg var values: String)
class HiddenAfter(vararg private val values: String) { fun count(): Int = values.size }
class HiddenBefore(private vararg val values: String) { fun count(): Int = values.size }
class Bare(vararg values: Int) { val count = values.size }
class Default(val prefix: Int = 7, vararg val values: Int)
open class Parent(vararg val values: Int)
class Child : Parent(2, 3)
class SpreadChild(input: IntArray) : Parent(*input)
class Delegating(vararg val values: Int) {
    constructor(first: Int) : this(first, first + 1, first + 2)
    constructor(marker: Boolean, input: IntArray) : this(*input)
}
object Trace {
    var log: Int = 0
    fun next(value: Int): Int { log = log * 10 + value; return value }
}
fun empty(): Strings = Strings("e")
fun packed(): Strings = Strings("p", "a", "bb")
fun spread(input: Array<String>): Strings = Strings("s", "before", *input, "after")
fun primitive(input: IntArray): Ints = Ints(0, *input, 4)
fun wide(): Wide = Wide(4294967298L, 4294967299L)
fun nullable(): Nullable = Nullable(null, "n")
fun generic(): Generic<String> = Generic<String>("x", "yy")
fun genericInts(): Generic<Int> = Generic<Int>(1, 2)
fun defaults(): Default = Default(values = intArrayOf(1, 2))
fun effects(): Ints {
    Trace.log = 0
    return Ints(Trace.next(1), *intArrayOf(Trace.next(2), Trace.next(3)), Trace.next(4))
}
