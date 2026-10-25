package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val value = DelegatedBox(); value.score = 6; check(value.score == 6); println("OK") }
