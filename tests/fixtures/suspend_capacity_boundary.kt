package suspendcapacityboundary

suspend fun nine(step: suspend (Int) -> Int): Int {
    val a = step(1)
    val b = step(a + 1)
    val c = step(b + 1)
    val d = step(c + 1)
    val e = step(d + 1)
    val f = step(e + 1)
    val g = step(f + 1)
    val h = step(g + 1)
    val i = step(h + 1)
    return i
}
