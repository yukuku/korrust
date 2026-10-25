package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(literals() == 4294935864L); check(contextualLong() == 42L); println("OK") }
