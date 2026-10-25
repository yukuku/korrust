package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(state() == 35); check(StateClass.value == 5); println("OK") }
