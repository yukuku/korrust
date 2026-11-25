private fun escaped(log: IntArray): Int {
    sourceBracket<Int>({ return 37 }, { log[0] = log[0] * 10 + 4 })
    return 0
}

fun main() {
    val log = intArrayOf(0)
    check(escaped(log) == 37)
    check(log[0] == 4)
    check(updateSource(9) == 9)
    check(localShadow(4) == 5 && sourceState == 9)
    println("4 consumer checks")
}
