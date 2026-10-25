package declarations.probes
import declarations.support.Trace
class SecondaryBox(val value: Int) { init { Trace.mark(1) }; constructor(): this(4) { Trace.mark(2) }; constructor(text: String): this() { Trace.mark(3) } }
fun secondary(): Int = SecondaryBox().value + SecondaryBox("abc").value
class SecondaryCollision { var stored: Int = 0; init { val scratch = 7; stored = scratch }; constructor(value: Int): super() { stored += value } }
