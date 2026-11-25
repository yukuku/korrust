open class Base<T>(val value: T)
fun <T> make(value: T): Base<T> = object : Base<T>(value) {}
