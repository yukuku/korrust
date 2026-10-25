package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { check(receiverLambda() == "a23"); val receiver = StringBuilder(); sourceReceiver(receiver) { amount -> append("x"); this.append(amount) }; check(receiver.toString() == "x4"); println("OK") }
