@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
class Host { companion object { val code: Int = read(V(1)) } }
