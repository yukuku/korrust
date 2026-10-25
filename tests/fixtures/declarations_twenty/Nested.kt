package declarations.probes
import declarations.support.Trace
class Outer { class Nested(val value: Int); object Holder { val value = 8 }; fun compute(): Int = Nested(2).value + Holder.value }
fun nested(): Int = Outer().compute()
