@JvmInline value class V(val raw: Int)
enum class E(val value: V) { A(V(1)) }
