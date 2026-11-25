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

fun sourceTryReturn(log: IntArray): Int {
    sourceBracket<Int>({
        try {
            return 83
        } finally {
            log[0] = log[0] * 10 + 2
        }
    }, { log[0] = log[0] * 10 + 3 })
    return 0
}

var sourceState: Int = 0
inline fun updateSource(value: Int): Int {
    sourceState = value
    return sourceState
}
fun sourceProperty(): Int = updateSource(5)

inline fun localShadow(value: Int): Int {
    var sourceState = value
    sourceState = sourceState + 1
    return sourceState
}
fun sourceLocalShadow(): Int {
    sourceState = 77
    return localShadow(2) * 100 + sourceState
}
fun sourceCapturedShadow(): Int {
    var sourceState = 2
    val result = sourceBracket<Int>({ sourceState = sourceState + 1; sourceState }, {})
    return result * 10 + sourceState
}
