@JvmInline value class V(val raw: Int)
suspend fun bounded(step: suspend () -> Int): Int { val saved = V(1); val result = step(); return saved.raw + result }