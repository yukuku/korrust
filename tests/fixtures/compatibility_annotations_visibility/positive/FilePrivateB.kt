package compatibility.visibility

private val scopedValue: Int = 8
private fun scopedAdd(value: Int): Int = value + scopedValue
fun privateB(): Int = scopedAdd(2)
