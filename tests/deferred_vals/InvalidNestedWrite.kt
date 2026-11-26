package deferredvals
fun nestedWrite(flag: Boolean): Int {
    val answer: Int
    answer = if (flag) { answer = 3; 4 } else 5
    return answer
}
