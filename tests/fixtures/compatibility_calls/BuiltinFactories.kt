package compatibility.calls.factories
fun builtinFactories(): String {
    val values = intArrayOf(2, 3)
    val mixed = intArrayOf(1, *values, 4)
    val copied = intArrayOf(*values)
    copied[0] = 8
    return "${values.size}:${values[0]}:${values[1]}:${mixed.size}:${mixed[0] + mixed[1] + mixed[2] + mixed[3]}:${copied[0]}"
}
