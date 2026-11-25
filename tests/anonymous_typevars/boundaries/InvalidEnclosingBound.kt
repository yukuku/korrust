open class Bound<X : Number>(val value: X)
fun <T : CharSequence> invalid(value: T): Bound<T> = object : Bound<T>(value) {}
