package trailing.dependency
fun <T, R> T.convert(prefix: String = "dep", count: Int = 1, block: (T) -> R): String =
    prefix + count + ":" + block(this)
fun <T, R> apply(value: T, pad: String = "dep", block: ((T) -> R)? = null): R? = block?.invoke(value)
class Adapter {
    fun <T> dispatch(value: T, label: String = "member", block: (T) -> String): String = label + block(value)
}
fun overloaded(label: String = "string", block: () -> String): String = label + block()
fun overloaded(value: Int, block: () -> String): String = value.toString() + block()
