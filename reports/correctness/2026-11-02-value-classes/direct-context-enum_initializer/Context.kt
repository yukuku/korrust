package directcontext
@JvmInline value class V(val raw: Int)
enum class E(val code: Int) { A(V(17).raw) }
