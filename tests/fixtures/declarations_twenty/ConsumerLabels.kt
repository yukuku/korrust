package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(labeledLoops() == 6); check(labeledReturn() == 9); println("OK") }
