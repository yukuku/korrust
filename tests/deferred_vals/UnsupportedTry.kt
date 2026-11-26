package deferredvals
fun once(): Int {
    val answer: Int
    try { answer = 3 } finally { }
    return answer
}
