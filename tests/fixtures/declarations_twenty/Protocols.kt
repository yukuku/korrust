package declarations.library
import declarations.support.Trace
import kotlin.reflect.KProperty
class Bag<T>(val value: T) { operator fun contains(other: T): Boolean { Trace.mark(1); return value == other } }
class Slots<T>(var value: T) {
    operator fun get(row: Int, column: Int): T { Trace.mark(4); if(row != 0 || column != 0) throw IndexOutOfBoundsException(); return value }
    operator fun set(row: Int, column: Int, input: T) { Trace.mark(5); if(row != 0 || column != 0) throw IndexOutOfBoundsException(); value = input }
    operator fun get(index: Int): T { Trace.mark(2); if (index != 0) throw IndexOutOfBoundsException(); return value }
    operator fun set(index: Int, input: T) { Trace.mark(3); if (index != 0) throw IndexOutOfBoundsException(); value = input }
}
class Components {
    operator fun component1(): Int { Trace.mark(1); return 4 }
    operator fun component2(): String { Trace.mark(2); return "abc" }
}
class Cell<T>(var value: T) {
    private var expectedOwner: Any? = null
    private var provisions: Int = 0
    operator fun provideDelegate(owner: Any?, property: KProperty<*>): Cell<T> {
        Trace.mark(3)
        if (property.name != "score" || provisions != 0 || owner == null) throw IllegalStateException("provideDelegate")
        provisions += 1
        expectedOwner = owner
        return this
    }
    operator fun getValue(owner: Any?, property: KProperty<*>): T { Trace.mark(1); if (property.name != "score" || owner !== expectedOwner || provisions != 1) throw IllegalStateException("owner/name/count"); return value }
    operator fun setValue(owner: Any?, property: KProperty<*>, input: T) { Trace.mark(2); if (property.name != "score" || owner !== expectedOwner || provisions != 1) throw IllegalStateException("owner/name/count"); value = input }
}
fun withReceiver(value: StringBuilder, block: StringBuilder.() -> Unit) { value.block() }
fun invokeInt(block: () -> Int): Int = block()
class OrdinaryContains { fun contains(value: Int): Boolean = value == 1 }
class OrdinaryIndex { fun get(index: Int): Int = index; fun set(index: Int, value: Int) {} }
fun withReceiverArgument(value: StringBuilder, block: StringBuilder.(Int) -> Unit) { value.block(3) }
private inline fun concealed(value: Int): Int = value + 100
open class ProtectedInline { protected inline fun guarded(value: Int): Int = value + 1 }
