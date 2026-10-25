package declarations.consumer
import declarations.probes.*
import declarations.library.*
fun main() { val values: Array<String> = makeArray(); check(values[2] == "v2"); check(arrayTotal(arrayOf<String?>(null, "ok")) == 2); println("OK") }
