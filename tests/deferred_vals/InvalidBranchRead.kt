package deferredvals
fun missing(flag: Boolean): Int {
    val answer: Int
    if (flag) answer = 3 else answer = answer + 1
    return answer
}
