package listops

fun concatenate(left: List<String>, right: List<String>, last: String): List<String> =
    left + right + last
fun without(left: MutableList<Int>, right: List<Int>): List<Int> = left - right
fun custom(left: List<Int>, times: Int): Int = left * times
