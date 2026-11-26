package deferredvals
fun once(): Int {
    val answer: Int
    while (true) { answer = 3; break }
    return answer
}
