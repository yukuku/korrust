package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(tryValue(value = 0) == 7); check(finallyOverride() == 9); check(finallyBypass() == 8); check(nestedCatchFinally() == 5); check(tryReference(false).compute() == 5); check(mutableReference(true).compute() == 9); println("OK") }
