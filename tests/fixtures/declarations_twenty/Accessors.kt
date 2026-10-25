package declarations.probes
import declarations.support.Trace
class AccessorBox { var value: Int = 2
get() { Trace.mark(1); return field * 2 }
set(input) { Trace.mark(2); field = input + 1 }
val computed: Int get() = value + 3 }
fun accessorValue(): Int { val box = AccessorBox(); box.value = 4; return box.computed }
