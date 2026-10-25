package next.probes
import next.library.*
import next.library.weight as measured

fun arrayIndices(values: IntArray): Int { var n = 0; for (i in values.indices) { n += i * values[i] }; return n }
fun stringIndices(value: String): Int { var n = 0; for (i in value.indices) { n += i * value[i].code }; return n }
fun listIndices(value: List<String>): Int { var n = 0; for (i in value.indices) { n += i * value[i].length }; return n }
fun extensionProperty(value: String): Int = value.measured + value.mass
