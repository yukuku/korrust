@JvmInline value class V(val raw: Int)
fun read(value: V): Int = value.raw
enum class E { A { override fun body(): Int = read(V(1)) }; abstract fun body(): Int }
