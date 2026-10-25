package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(membership(Bag<String>("yes"), "yes")); check(concreteMembership(7)); println("OK") }
