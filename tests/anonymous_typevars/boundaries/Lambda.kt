open class Base<X>(val seed: X)
fun <T> make(value: T): () -> Base<T> = { object : Base<T>(value) {} }
