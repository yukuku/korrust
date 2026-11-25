suspend fun choose(mode: Int, step: suspend (Int) -> Int): Int {
    val saved = 7
    val selected = if (mode < 0) 5 else if (mode == 0) step(10) else step(20)
    val result = when (mode) {
        0 -> {
            val next = step(selected)
            next
        }
        1 -> selected + 1
        else -> selected
    }
    return result + saved
}

suspend fun loops(limit: Int, step: suspend (Int) -> Int): Int {
    var i = 0
    var total = 0
    outer@ while (i < limit) {
        i++
        if (i == 2) continue
        var j = 0
        do {
            j++
            val next = step(i * 10 + j)
            total += next
            if (j == 2) break
        } while (j < 3)
        if (i == 3) break@outer
    }
    for (n in 0 until 3) {
        if (n == 1) continue
        val next = step(100 + n)
        total += next
    }
    return total + i
}

suspend fun array(values: IntArray, step: suspend (Int) -> Int): Int {
    var total = 0
    for (value in values) {
        val next = step(value)
        total += next
    }
    return total
}

suspend fun unit(step: suspend () -> Unit) {
    if (true) return step()
}

suspend fun localBranch(mode: Boolean, step: suspend (Int) -> Int): Int {
    val saved = 4
    suspend fun inner(): Int = if (mode) step(saved) else saved
    return inner()
}

suspend fun condition(enabled: Boolean, step: suspend () -> Boolean): Int {
    if (enabled && step()) return 11
    return 22
}
