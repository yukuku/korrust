open class Base<T>(val value: T)
fun bad(): Base<Int> = object : Base<Int>(2) {}
