package declarations.probes
import declarations.support.Trace
import declarations.library.Components
fun destructure(value: Components): Int { val (a, b) = value; return a + b.length }
fun destructureSkip(value: Components): String { val (_, b) = value; return b }
fun destructureLoop(values: List<Components>): Int { var total = 0; for((a, b) in values) { total += a + b.length }; return total }
fun destructureLambda(value: Components): Int { val add: (Components) -> Int = { (a, b) -> a + b.length }; return add(value) }
