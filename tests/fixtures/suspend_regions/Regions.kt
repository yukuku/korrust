class Trace {
    var text: String = ""
    fun add(value: String) { text = text + value }
}

suspend fun caught(step: suspend (String) -> String, trace: Trace): String {
    try {
        return step("body")
    } catch (error: IllegalStateException) {
        trace.add("C")
        return step("catch")
    } finally {
        trace.add("F")
    }
}

suspend fun returning(step: suspend (String) -> String, trace: Trace): Long {
    try {
        return 9000000001L
    } finally {
        trace.add("B")
        step("finally")
        trace.add("A")
    }
}

suspend fun expression(step: suspend (String) -> String, trace: Trace): String {
    val result = try {
        step("body")
    } finally {
        trace.add("B")
        step("finally")
        trace.add("A")
    }
    return result
}

suspend fun recovering(step: suspend (String) -> String, trace: Trace): String {
    try {
        return "original"
    } finally {
        try {
            try {
                return "override"
            } finally {
                trace.add("N")
                step("nested")
            }
        } catch (error: IllegalStateException) {
            trace.add("C")
            val ignored = step("recovered")
        }
        trace.add("F")
    }
}

suspend fun loop(step: suspend (String) -> String, trace: Trace): String {
    var index = 0
    while (index < 3) {
        index = index + 1
        try {
            step("body")
            if (index == 1) continue
            break
        } finally {
            trace.add("B")
            step("finally")
            trace.add("A")
        }
    }
    return trace.text
}
