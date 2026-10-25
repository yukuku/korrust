package declarations.probes
import declarations.support.Trace
import declarations.library.Slots
fun readSlot(values: Slots<String>, index: Int): String = values[index]
fun writeSlot(values: Slots<String>, value: String) { values[Trace.mark(0)] = value }
class ConcreteSlots { var value = 0; operator fun get(index: Int): Int = value + index; operator fun set(index: Int, input: Int) { value = input + index } }
fun concreteSlots(): Int { val values = ConcreteSlots(); values[2] = 5; return values[3] }

fun multiSlot(values: Slots<String>): String { values[Trace.mark(1) - 1, Trace.mark(2) - 2] = "multi"; return values[0, 0] }
