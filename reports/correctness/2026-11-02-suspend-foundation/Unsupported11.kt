@JvmInline value class V(val raw: Int)
suspend fun bounded(value: V): V = value