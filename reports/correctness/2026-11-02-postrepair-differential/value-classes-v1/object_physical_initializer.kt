@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
object O { val code: Int = read(V(1)) }
