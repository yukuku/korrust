package compatibility.visibility
private class Hidden(val value: Int)
fun hiddenValue(): Int = Hidden(3).value
