package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun boundReferenceProbe(value: Counter): Int { val block: (Int) -> Int = value::add; return block(4) }
fun unboundReferenceProbe(value: Counter): Int { val block: (Counter, Int) -> Int = Counter::add; return block(value, 5) }
fun receiverReferenceProbe(): Int { val block: (Int) -> Int = counterReceiver()::add; return block(1) + block(2) }
