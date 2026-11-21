package inference
import inference.dependency.*
import inference.dependency.exists
import inference.dependency.exists as available
import java.util.regex.Pattern
fun early(flag: Boolean): Array<String> {
    if (flag) return emptyArray()
    return arrayOf("present")
}
fun guarded(value: String?): Int {
    if (!exists(value)) return -1
    return value.length
}
fun reversed(value: String?): Int {
    if (absent(value)) return -1
    return value.length
}
fun receiver(value: String?): Int {
    if (value.missing()) return -1
    return value.length
}
fun typeResult(value: Any?): Int {
    if (stringValue(value)) return value.length
    return -1
}
fun both(a: String?, b: String?): Int {
    if (pair(b = b, a = a)) return a.length + b.length
    return -1
}
fun standard(value: String?): Int {
    if (value.isNullOrEmpty()) return -1
    return value.length
}
fun standardBoolean(value: String?): Int {
    if (value.isNullOrEmpty() == false) return value.length
    return -1
}
fun initialized(flag: Boolean): Array<String> {
    val result: Array<String> = if (flag) emptyArray() else arrayOf("value")
    return result
}
fun caught(flag: Boolean): Array<String> = try {
    if (flag) throw IllegalArgumentException("empty")
    arrayOf("value")
} catch (error: IllegalArgumentException) {
    emptyArray()
}
fun matchers(): Array<java.util.regex.Matcher> = Array(2) { Pattern.compile("a").matcher("") }
fun bound(value: String): String? = value.choose(value.length == 0) { null }
fun standardBound(value: String): String? = value.ifEmpty { null }
fun contextExtension(value: String): Array<String> = value.created()
fun selection(value: String?): Int {
    if (selected(value, "good")) return value.length
    return -1
}

fun nullableEquality(value: String?): Int {
    if (nullableTruth(value) == true) return value.length
    return -1
}
fun nullableFalseEquality(value: String?): Int {
    if (nullableFalse(value) == false) return value.length
    return -1
}
fun nullableFunction(value: Int): String? = mapMaybe(value) { number -> number.toString() }
fun nullableFunctionRef(value: Int): String? = mapMaybe(value, ::label)
fun absentFunction(value: Int): String? = mapMaybe(value, null)
private fun label(value: Int): String = value.toString()
fun nullableJoined(values: List<Byte>): String = values.joinToString(separator = " ", transform = { "item" })
fun nullableJoinedFull(values: List<Byte>): String = values.joinToString(" ", "", "", -1, "...", { "item" })
fun directBoolean(value: String?): Int {
    if (exists(value)) return value.length
    return -1
}
fun equalsTrue(value: String?): Int {
    if (exists(value) == true) return value.length
    return -1
}
fun equalsTrueReverse(value: String?): Int {
    if (true == exists(value)) return value.length
    return -1
}
fun notEqualsFalse(value: String?): Int {
    if (exists(value) != false) return value.length
    return -1
}
fun notEqualsFalseReverse(value: String?): Int {
    if (false != exists(value)) return value.length
    return -1
}
fun equalsFalse(value: String?): Int {
    if (absent(value) == false) return value.length
    return -1
}
fun equalsFalseReverse(value: String?): Int {
    if (false == absent(value)) return value.length
    return -1
}
fun notEqualsTrue(value: String?): Int {
    if (absent(value) != true) return value.length
    return -1
}
fun notEqualsTrueReverse(value: String?): Int {
    if (true != absent(value)) return value.length
    return -1
}
fun nullableEqualityReverse(value: String?): Int {
    if (true == nullableTruth(value)) return value.length
    return -1
}
fun nullableFalseEqualityReverse(value: String?): Int {
    if (false == nullableFalse(value)) return value.length
    return -1
}
fun shortCircuitAnd(value: String?): Int {
    if (exists(value) && value.length >= 0) return value.length
    return -1
}
fun shortCircuitOr(value: String?): Int {
    if (absent(value) || value.length == 0) return -1
    return value.length
}
fun ordinaryAny(value: String?): Any = anyFalse(value)
fun importedAlias(value: String?): Int {
    if (available(value)) return value.length
    return -1
}
