package deferredvals

fun choose(flag: Boolean): Int {
    val answer: Int
    if (flag) answer = 3 else answer = 4
    return answer
}
fun continuing(flag: Boolean): Int {
    val answer: Int
    if (flag) { answer = 5; return answer }
    answer = 8
    return answer
}
fun expression(flag: Boolean): Int {
    val answer: Int
    val text = if (flag) { answer = 5; "a" } else { answer = 6; "b" }
    return answer + text.length
}
fun selected(value: Int): Int {
    val answer: Int
    when (value) { 0 -> answer = 10; 1 -> answer = 20; else -> answer = 30 }
    return answer
}
fun selectedExpression(flag: Boolean): Int {
    val answer: Int
    val add = when (flag) { true -> { answer = 11; 1 }; else -> { answer = 22; 2 } }
    return answer + add
}
fun nested(limit: Int): Int {
    var sum = 0
    for (index in 0 until limit) {
        val start: Int
        val end: Int
        if (index % 2 == 0) { start = index; end = index + 1 }
        else { start = index + 2; end = start + 1 }
        sum = sum + start + end
    }
    return sum
}
fun shadow(flag: Boolean): Int {
    val answer: Int
    var total = 0
    if (flag) { val answer = 3; total = answer }
    answer = 9
    return answer + total
}
fun reference(flag: Boolean): String {
    val answer: String
    if (flag) answer = "left" else answer = "right"
    return answer
}
fun wide(flag: Boolean): Long {
    val answer: Long
    if (flag) answer = 4294967298L else answer = 4294967299L
    return answer
}
fun terminal(flag: Boolean): Int {
    val answer: Int
    if (flag) answer = return 7 else answer = 8
    return answer
}
fun capture(flag: Boolean): () -> Int {
    val answer: Int
    if (flag) answer = 9 else answer = 10
    return { answer }
}
