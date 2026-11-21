package trailing
import trailing.dependency.*
private var trace: String = ""
private fun note(value: String): String {
    trace += value
    return value
}
private fun source(prefix: String = "source", count: Int = 2, block: () -> String): String = prefix + count + block()
private fun <T, R> transform(value: T, count: Int = 3, block: (T) -> R): R = block(value)
private inline fun immediate(prefix: String, block: () -> String): String = prefix + block()
fun sourceDefault(): String = source { "!" }
fun sourceNamed(): String = source(count = 4) { "!" }
fun sourceGeneric(): String = transform(7) { it.toString() }
fun compiledExtension(): String = 5.convert { it.toString() }
fun compiledNamed(): String = 5.convert(count = 6) { it.toString() }
fun compiledNullable(): String? = apply(8) { it.toString() }
fun compiledMember(): String = Adapter().dispatch(9) { it.toString() }
fun compiledOverload(): String = overloaded { "!" }
fun selectedOverload(): String = overloaded(4) { "!" }
fun explicitLabel(): String = source named@{ return@named "label" }
fun implicitLabel(): String = source { return@source "label" }
fun inlineReturn(): String {
    immediate("inline") { return "early" }
    return "late"
}
fun joined(values: List<Byte>): String = values.joinToString(" ") { "%02x".format(it) }
fun ordered(): String {
    trace = ""
    val result = source(count = 4, prefix = note("A")) { note("B") }
    return trace + ":" + result
}
fun arrayInitializer(): String = Array(1) { "array" }[0]
internal class Holder(val block: () -> String)
fun constructed(): String {
    val holder = Holder { "constructor" }
    val block = holder.block
    return block()
}
fun functionValue(): String {
    val call: (Int, () -> String) -> String = { value, block -> value.toString() + block() }
    return call(2) { "function" }
}
fun captured(): String {
    val suffix = "captured"
    return source { suffix }
}
