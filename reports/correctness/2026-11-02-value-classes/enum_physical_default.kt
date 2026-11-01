@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
enum class E(val code: Int = read(V(1))) { A }
