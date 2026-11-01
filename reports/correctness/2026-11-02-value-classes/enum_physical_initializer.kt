@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
enum class E(val code: Int) { A(read(V(1))) }
