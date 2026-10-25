package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val value = LateBox(); value.text = "ok"; check(value.text == "ok"); println("OK") }
