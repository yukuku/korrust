package external.nested
fun <T> pick(value: T): T = value
fun test(value: Int): Int = value + 2
fun test(value: String): String = value + "!"
private fun hidden(): Int = 20
