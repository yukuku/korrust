package deferredvals
fun repeated(): Int {
    val answer: Int
    answer = 3
    val action: () -> Unit = { answer = 4 }
    action()
    return answer
}
