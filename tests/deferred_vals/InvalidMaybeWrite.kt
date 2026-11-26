package deferredvals
fun repeated(flag: Boolean): Int {
    val answer: Int
    if (flag) answer = 3
    answer = 4
    return answer
}
