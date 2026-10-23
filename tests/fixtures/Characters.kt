package korrust.probes

import fixtures.WhenTracker

fun charIdentity(value: Char): Char = value
fun charLocal(value: Char): Char {
    var result: Char = '\u0000'
    result = value
    return result
}
fun charArrayEcho(text: CharArray): CharArray = text
fun charRead(text: CharArray, index: Int): Char = text[index]
fun charSize(text: CharArray): Int = text.size
fun charEqual(a: Char, b: Char): Boolean = a == b
fun charUnequal(a: Char, b: Char): Boolean = a != b
fun charLess(a: Char, b: Char): Boolean = a < b
fun charLessEqual(a: Char, b: Char): Boolean = a <= b
fun charGreater(a: Char, b: Char): Boolean = a > b
fun charGreaterEqual(a: Char, b: Char): Boolean = a >= b
fun charLiteral(which: Int): Char {
    when (which) {
        0 -> return '\u0000'
        1 -> return '\uD800'
        2 -> return '\uDFFF'
        3 -> return '\t'
        4 -> return '\b'
        5 -> return '\n'
        6 -> return '\r'
        7 -> return '\''
        8 -> return '\"'
        9 -> return '\\'
        10 -> return '\$'
        11 -> return '$'
        12 -> return '漢'
        else -> return '\uFFFF'
    }
}
fun addAssign(a: Int, b: Int, c: Int): Int {
    var result = a
    result += b
    result += c
    return result
}
fun intWhen(tracker: WhenTracker): Int {
    when (tracker.subject()) {
        tracker.first(), tracker.second() -> return 11
        tracker.third() -> return 13
        else -> return 17
    }
}
fun charWhen(text: CharArray, tracker: WhenTracker): Int {
    when (text[tracker.subject()]) {
        text[tracker.first()], text[tracker.second()] -> return 11
        text[tracker.third()] -> return 13
        else -> return 17
    }
}
fun whenScopes(value: Int): Int {
    var result = 1
    when (value) {
        0, 1 -> {
            val result = value + 4
            when (result) {
                4 -> return 40
                else -> return result
            }
        }
        2 -> {
            val value = 8
            result += value
        }
        else -> { result += 10 }
    }
    return result
}
fun whenFallthrough(value: Int): Int {
    var result = 10
    when (value) {
        1 -> { result += 3 }
        2, 3 -> { result += 7 }
    }
    return result
}
fun emptyWhen(tracker: WhenTracker): Int {
    when (tracker.subject()) {}
    return 29
}
