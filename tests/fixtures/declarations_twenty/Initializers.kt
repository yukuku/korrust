package declarations.probes
import declarations.support.Trace
class InitBox(val input: Int) { val first = Trace.mark(input); init { Trace.mark(2) }; val second = Trace.mark(3); init { Trace.mark(4) }; fun sum(): Int = first + second }
fun initializers(): Int = InitBox(1).sum()
