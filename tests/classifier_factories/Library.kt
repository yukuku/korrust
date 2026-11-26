package factorylib
interface Value { fun value(): Int }
private class StoredValue(val stored: Int) : Value { override fun value(): Int = stored }
fun Value(seed: Int): Value = StoredValue(seed)
interface Slot<T> { fun value(): T }
private class StoredSlot<T>(val stored: T) : Slot<T> { override fun value(): T = stored }
fun <T> Slot(value: T): Slot<T> = StoredSlot(value)
class Box(val value: Int)
fun Box(text: String): Box = Box(text.length)
class Decision(val value: CharSequence)
fun Decision(value: java.io.Serializable): Decision = Decision(value.toString() as CharSequence)
