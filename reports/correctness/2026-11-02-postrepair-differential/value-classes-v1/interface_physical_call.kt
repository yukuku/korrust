@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
interface I { fun body(): Int = read(V(1)) }
