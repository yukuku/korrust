fun pending(x: Int?): Int {
    var total = 0
    var i = 0
    while (i < 3) {
        i += 1
        total += 1 + (x ?: continue)
    }
    return total
}

private fun add(first: Long, second: Long): Long = first + second

fun wide(values: Array<String?>): Long {
    var total = 0L
    var visits = 0L
    for (value in values) {
        visits += 1L
        total += add(visits, (value ?: continue).length.toLong())
    }
    return total + visits
}
