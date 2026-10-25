package declarations.probes
import declarations.support.Trace
object StateObject { var value = Trace.mark(2); fun add(): Int { value += 1; return value } }
class StateClass { companion object { var value = Trace.mark(3); fun add(): Int { value += 2; return value } } }
fun state(): Int = StateObject.add() * 10 + StateClass.add()
