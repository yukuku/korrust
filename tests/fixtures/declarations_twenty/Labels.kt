package declarations.probes
import declarations.support.Trace
import declarations.library.invokeInt
fun labeledLoops(): Int { var total = 0; outer@ for (x in 1..3) { for (y in 1..3) { if (y == 2) { try { continue@outer } finally { Trace.mark(1) } }; total += x } }; return total }
fun labeledBreak(): Int { var total = 0; outer@ while (total < 10) { while (true) { total += 1; try { break@outer } finally { Trace.mark(2) } } }; return total }
fun labeledReturn(): Int = invokeInt block@ { return@block 9 }
