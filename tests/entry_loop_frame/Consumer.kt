class Worker {
    fun drain() { while (Loop.next()) Unit }
}
fun runWide(x: Long): Long { while (Loop.next()) { if (x < 0) return x }; return x + 1 }
