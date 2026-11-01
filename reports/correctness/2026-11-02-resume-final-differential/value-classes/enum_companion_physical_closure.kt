@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
enum class E { A; companion object { fun body(): Int { val result: () -> Int = { read(V(1)) }; return result() } } }
