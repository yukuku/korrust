package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(SecondaryBox().value == 4); check(SecondaryBox(text = "abc").value == 4); check(SecondaryCollision(value = 5).stored == 12); println("OK") }
