package transform
fun <T,R> transform(first: T, block: (T) -> R): R = block(first)
class Words {
    fun pick(value: Int): String = "n$value"
    fun pick(value: String): Int = value.length
}
