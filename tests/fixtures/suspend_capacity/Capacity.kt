package suspendcapacity

var trace: String = ""

suspend fun eight(step: suspend (Int) -> Int): Int {
    val a = step(1)
    trace += "a"
    val b = step(a + 1)
    trace += "b"
    val c = step(b + 1)
    trace += "c"
    val d = step(c + 1)
    trace += "d"
    val e = step(d + 1)
    trace += "e"
    val f = step(e + 1)
    trace += "f"
    val g = step(f + 1)
    trace += "g"
    val h = step(g + 1)
    trace += "h"
    return h + 1000
}
