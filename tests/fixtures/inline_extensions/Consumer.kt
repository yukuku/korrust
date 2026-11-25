import extensions.measure
import extensions.measureLong
import extensions.fallback
import extensions.accepts

fun main() {
    check("abc".measure { it + 2 } == 5)
    check(4L.measureLong { it + 7L } == 11L)
    val empty: String? = null
    check(empty.fallback { 13 } == 13)
    check("type".accepts<String>("text"))
    check(!"type".accepts<String>(7))
    println("OK")
}
