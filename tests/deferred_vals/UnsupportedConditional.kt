package deferredvals
fun once(flag: Boolean): Int {
    val answer: Int
    flag && if (flag) { answer = 3; true } else { answer = 4; false }
    return 0
}
