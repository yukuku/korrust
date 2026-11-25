inline fun <T> sourceBracket(body: () -> T, after: () -> Unit): T {
    try {
        return body()
    } finally {
        after()
    }
}

fun sourceReturn(log: IntArray): Int {
    sourceBracket<Int>({ return 81 }, { log[0] = log[0] * 10 + 3 })
    return 0
}
