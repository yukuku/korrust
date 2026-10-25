package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(destructure(Components()) == 7); check(destructureLoop(listOf(Components())) == 7); check(destructureLambda(Components()) == 7); println("OK") }
