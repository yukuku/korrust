package deferredvals
fun early(): Int {
    val answer: Int
    val action: () -> Int = { answer }
    answer = 3
    return action()
}
