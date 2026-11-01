package suspension

import suspend_dependency.fetch

@JvmInline
value class Unrelated(val value: Int)

fun valueNeighbor(input: Int): Int = Unrelated(input).value

var trace: String = ""

suspend fun combine(input: Int, first: suspend (Int) -> String, second: suspend (String) -> Long): String {
    val before = input * 2
    val text = first(input)
    trace += "a"
    val tag = text + "-" + before
    val number = second(tag)
    trace += "b"
    return tag + ":" + number
}

suspend fun leaf(input: Int, step: suspend (Int) -> Int): Int {
    val answer = step(input)
    return answer + 10
}

suspend fun outer(seed: Int, step: suspend (Int) -> Int): Int {
    val before = seed * 2
    val got = leaf(before, step)
    return got + 1
}

suspend fun imported(input: String, step: suspend (String) -> String): String {
    val got = fetch(input, step)
    return "[" + got + "]"
}

suspend fun wide(input: Long, fraction: Double, step: suspend (Double) -> Double): Double {
    val before = input.toDouble() + fraction
    val got = step(before)
    return got + before
}

suspend fun nullable(step: suspend () -> String?): String? {
    val result = step()
    trace += "n"
    return result
}

suspend fun unit(step: suspend () -> Unit) {
    step()
    trace += "u"
}

suspend fun immediate(input: String): String = input

private suspend fun hidden(step: suspend () -> String): String {
    val result = step()
    return result + "hidden"
}

suspend fun privatePath(step: suspend () -> String): String {
    val result = hidden(step)
    return result + "!"
}

suspend fun throwAfter(step: suspend () -> String, error: RuntimeException): String {
    val value = step()
    trace += value
    throw error
}

suspend fun mutation(step: suspend (Int) -> Int): Int {
    var number = 3
    val value = step(number++)
    return number * 10 + value
}

suspend fun unitTail(step: suspend () -> Unit): Unit = step()

fun receive(value: Int?): Int? = value
fun ordinaryBox(value: Int): Any = value

suspend fun boxingPositions(value: Int, step: suspend () -> Unit): Array<Any?> {
    val boxed: Any = value
    val values = listOf(value)
    val nullable = receive(value)
    val ordinary = ordinaryBox(value)
    step()
    return arrayOf(boxed, values[0], nullable, ordinary)
}
