package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(Outer.Nested(value = 4).value == 4); check(Outer.Holder.value == 8); println("OK") }
