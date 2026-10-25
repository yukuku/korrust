package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(before() == 9); check(MixedBox(value = 4).value == 4); println("OK") }
