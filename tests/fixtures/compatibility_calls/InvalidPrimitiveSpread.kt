package compatibility.calls.invalid
import compatibility.calls.lib.kotlinInts
fun bad(values: Array<Int>): Int = kotlinInts(*values)
