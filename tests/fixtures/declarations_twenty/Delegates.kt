package declarations.probes
import declarations.support.Trace
import declarations.library.Cell
class DelegatedBox { var score: Int by Cell<Int>(3) }
fun delegatedValue(): Int { val box = DelegatedBox(); box.score = 8; return box.score }
