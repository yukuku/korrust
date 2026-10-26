package compatibility.calls
fun intArrayOf(value: Int): IntArray {
    val values = IntArray(1)
    values[0] = value + 100
    return values
}
fun builtinShadow(): String = "${intArrayOf(3)[0]}"
