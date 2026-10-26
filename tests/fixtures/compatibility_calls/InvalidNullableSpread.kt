package compatibility.calls.invalid
import compatibility.calls.lib.strictStrings
fun bad(values: Array<String?>): Int = strictStrings(*values)
