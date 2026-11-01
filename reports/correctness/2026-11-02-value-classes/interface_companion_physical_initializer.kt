@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
interface I { companion object { val code: Int = read(V(1)) } }
