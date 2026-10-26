package compatibility.visibility

private val scopedValue: Int = 5
private fun scopedAdd(value: Int): Int = value + scopedValue
fun privateA(): Int = scopedAdd(1)
