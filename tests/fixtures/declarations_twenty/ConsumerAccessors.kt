package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val value = AccessorBox(); value.value = 4; check(value.computed == 13); println("OK") }
