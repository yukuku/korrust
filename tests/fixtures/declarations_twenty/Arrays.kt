package declarations.probes
import declarations.support.Trace
fun makeArray(): Array<String> = Array<String>(3) { index -> Trace.mark(index); "v$index" }
fun arrayTotal(values: Array<String?>): Int { var total = 0; for (value in values) { if (value != null) total += value.length }; return total }
class ArrayItem(val value: Int)
fun integerArray(): Int { val values = Array<Int>(3) { it + 1 }; values[1] = 7; var total = 0; for(value in values) total += value; return total }
fun objectArray(): Array<ArrayItem> = Array<ArrayItem>(2) { ArrayItem(it + 4) }
