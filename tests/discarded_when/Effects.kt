var effectTrace: String = ""
fun mark(value: String): String { effectTrace += value; return value }
fun discardWhen(value: Int) {
    when (value) {
        0 -> 41
        1 -> mark("S")
        2 -> { effectTrace += "U" }
        else -> Unit
    }
}
fun discardBound(value: Int) {
    when (val subject = value) {
        1 -> mark(subject.toString())
        else -> false
    }
}
fun lambdaWhen(value: Int): () -> Unit = {
    when (value) {
        0 -> 13
        1 -> mark("L")
        else -> Unit
    }
}
fun orderedWhen(value: Int) {
    when (mark(value.toString())) {
        "1" -> mark("A")
        "2" -> 17
        else -> Unit
    }
}
fun branchReturn(value: Int): Int {
    when (value) {
        0 -> return 17
        1 -> mark("B")
        else -> Unit
    }
    return 19
}
fun readTrace(): String = effectTrace
fun resetTrace() { effectTrace = "" }
