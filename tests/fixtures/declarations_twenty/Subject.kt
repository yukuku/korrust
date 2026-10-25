package declarations.probes
import declarations.support.Trace
fun subjectVariable(input: Int): Int = when (val value = Trace.mark(input)) { 1 -> value + 10; else -> value + 20 }
