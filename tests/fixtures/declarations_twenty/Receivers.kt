package declarations.probes
import declarations.support.Trace
import declarations.library.withReceiver
import declarations.library.withReceiverArgument
fun receiverLambda(): String { val result = StringBuilder(); withReceiver(result) { append("a"); append(2) }; withReceiverArgument(result) { value -> this.append(value) }; return result.toString() }
fun sourceReceiver(value: StringBuilder, block: StringBuilder.(Int) -> Unit) { block(value, 4) }
