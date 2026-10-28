package sourcegeneric

class Box<T>(var value: T) {
    fun identity(other: T): T = other
}
interface Echo<T> {
    fun echo(value: T): T
}
class StringEcho : Echo<String> {
    override fun echo(value: String): String = value + "K"
}
fun box(): String {
    val box = Box<String>("O")
    box.value = box.identity("OK")
    return box.value
}
interface Relay<T> : Echo<T>
class GenericEcho<T> : Echo<T> {
    override fun echo(value: T): T = value
}
class IntEcho : Relay<Int> {
    override fun echo(value: Int): Int = value + 1
}
open class Parent<T>(val original: T) {
    open fun replace(value: T): T = value
}
class StringParent(original: String) : Parent<String>(original) {
    override fun replace(value: String): String = value + "K"
}
class Bounded<T : Number>(val value: T)
fun inherited(): String = StringParent("start").replace("O")
fun intEcho(): Int = IntEcho().echo(4)
fun genericEcho(): String = GenericEcho<String>().echo("OK")
fun nullableBox(): String? = Box<String?>(null).value
fun numberBox(): Int = Bounded<Int>(7).value
fun tailWhen(value: Int): Int {
    val answer = if (value > 0) {
        val next = value + 1
        when {
            next > 2 -> next
            else -> 2
        }
    } else {
        0
    }
    return answer
}
class LongEcho : Echo<Long> {
    override fun echo(value: Long): Long = value + 1L
}
class BooleanEcho : Echo<Boolean> {
    override fun echo(value: Boolean): Boolean = !value
}
class CharEcho : Echo<Char> {
    override fun echo(value: Char): Char = value
}
fun boxedEffects(): Int {
    val box = Box<Int>(Probe.next())
    return box.identity(Probe.next()) + box.value
}
fun inheritedProperty(): String = StringParent("OK").original
interface Slot<T> {
    var value: T
}
class StringSlot(override var value: String) : Slot<String>
class IntSlot(override var value: Int) : Slot<Int>
