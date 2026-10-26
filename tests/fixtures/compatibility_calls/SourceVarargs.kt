package compatibility.calls
fun sourceSum(vararg values: Int): Int { var sum = 0; for (value in values) sum += value; return sum }
fun sourceVarargs(): String = "${sourceSum()}:${sourceSum(4)}:${sourceSum(1, 2, 3)}"
