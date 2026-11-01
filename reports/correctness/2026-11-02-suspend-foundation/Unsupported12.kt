@JvmInline value class V(val raw: Int)
suspend fun bounded(step: suspend (V) -> Int): Int = step(V(1))