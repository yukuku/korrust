package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val slots = Slots<String>("v"); writeSlot(slots, "x"); check(readSlot(slots, 0) == "x"); println("OK") }
