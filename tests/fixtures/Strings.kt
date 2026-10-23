package korrust.probes

fun identity(value: String): String = value
internal fun internalIdentity(value: String): String = value
fun literal(): String = "ASCII\u0000é漢😀\uD800\uDC00\uD800X\uDFFF\t\b\n\r\'\"\\\$"
fun equal(a: String, b: String): Boolean = a == b
fun unequal(a: String, b: String): Boolean = a != b
fun length(value: String): Int = value.length
fun empty(value: String): Boolean = value.isEmpty()
fun remove(value: String, prefix: String): String = value.removePrefix(prefix)
fun starts(value: String, prefix: String): Boolean = value.startsWith(prefix)
fun ends(value: String, suffix: String): Boolean = value.endsWith(suffix)
fun suffix(value: String, start: Int): String = value.substring(start)
fun slice(value: String, start: Int, end: Int): String = value.substring(start, end)
fun compare(a: String, b: String): Int = a.compareTo(b)
fun concat(a: String, b: String, number: Int, flag: Boolean): String = a + b + number + flag
private fun stringDivide(a: Int, b: Int): Int = a / b
fun order(value: String, start: Int, divisor: Int): String = value.substring(start) + stringDivide(1, divisor)
private fun choose(value: String, flag: Boolean): String {
    var result: String = value
    if (flag) { result = result.removePrefix("ERROR_CODE_") } else { result = "fallback:" + result }
    return result
}
fun local(value: String, flag: Boolean): String = choose(value, flag)
fun internal(internal: String): String = internal
fun dollars(): String = "$1 $!"
