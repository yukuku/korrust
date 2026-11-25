package regions.dependency

inline fun <T> bracket(log: IntArray, digit: Int, block: () -> T): T {
    try {
        return block()
    } finally {
        log[0] = log[0] * 10 + digit
    }
}

inline fun recover(log: IntArray, block: () -> Int): Int {
    try {
        return block()
    } catch (error: IllegalArgumentException) {
        log[0] = log[0] * 10 + 2
        return 17
    } finally {
        log[0] = log[0] * 10 + 3
    }
}

inline fun nested(log: IntArray, block: () -> Int): Int {
    try {
        return bracket(log, 4) { block() }
    } finally {
        log[0] = log[0] * 10 + 5
    }
}

inline fun exploding(log: IntArray, block: () -> Int): Int {
    try {
        return block()
    } catch (error: IllegalStateException) {
        log[0] = log[0] * 10 + 8
        return 99
    } finally {
        log[0] = log[0] * 10 + 6
        throw IllegalStateException("cleanup")
    }
}

inline fun finish(log: IntArray, block: () -> Int, after: () -> Unit): Int {
    try {
        return block()
    } finally {
        after()
        log[0] = log[0] * 10 + 7
    }
}

inline fun <reified T> classify(value: Any?, log: IntArray): Boolean {
    try {
        return value is T
    } finally {
        log[0] = log[0] * 10 + 9
    }
}

inline fun guarded(lock: Any, log: IntArray, block: () -> Int): Int = synchronized(lock) {
    bracket(log, 2) { block() }
}
