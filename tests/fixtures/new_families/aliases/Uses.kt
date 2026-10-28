package newfamilies.aliases

class Holder(val transform: Transformer<String>, val rows: Rows<String>)

fun count(value: Rows<String>): Int = value.size
fun nullableCount(value: MaybeRows<String>): Int = value?.size ?: -1
fun apply(value: String?, transform: Transformer<String>): String = transform(value)
fun sum(value: Rows<Int>): Int = value[0] + value[1]
fun firstNested(value: Nested<String>): String? = value[0][0]
fun ignored(value: IgnoredChain<Int>): Ignored<String> = value
fun swap(value: Slots<String>): String? {
    val previous = value[0]
    value[0] = "changed"
    return previous
}
fun convert(value: Int, converter: Converter<Int, String>): String = converter(value)
fun format(value: String, formatter: Format<String>): String = value.formatter(3)
fun optional(value: OptionalTransform<String>): String = if (value == null) "none" else value("OK")
fun boxed(value: Boxed<String>): Boxed<String> = value
fun constrained(value: Constrained<String>): Constrained<String> = value

fun countTriple(value: TripleRows<String>): Int = value.size
fun countMutableNested(value: MutableRows<String>): Int = value.size
fun countMutableOuter(value: MutableOuterRows<String>): Int = value.size
fun countNullableNested(value: NullableRows<String>): Int = value.size
fun countFunctions(value: FunctionRows<Int>): Int = value.size
fun countBoxRows(value: BoxRows<String>): Int = value.size
fun countArrayRows(value: ArrayRows<String>): Int = value.size
fun <T> genericFirst(value: Rows<T>): T = value[0]
fun <T> genericApply(value: T?, transform: Transformer<T>): T = transform(value)
