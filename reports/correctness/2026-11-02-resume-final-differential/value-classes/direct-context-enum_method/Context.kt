package directcontext
@JvmInline value class V(val raw: Int)
enum class E { A; fun body(): Int { val value = V(13); return value.raw } }
