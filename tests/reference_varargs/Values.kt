package referencevarargs
class Item(val text: String)
fun join(vararg values: String): String {
    var result = ""
    for (value in values) result += value
    return result
}
fun nullable(vararg values: String?): Int {
    var total = 0
    for (value in values) if (value != null) total += value.length
    return total
}
fun objects(vararg values: Item): Int = values[0].text.length + values.size
fun capture(vararg values: String): () -> String = { values[0] }
fun expose(vararg values: String) = values
fun <T> generic(vararg values: T): T = values[0]
fun projection(values: Array<out CharSequence>): Int = values[0].length
fun forwarded(vararg values: String): String = join("x", *values, "z")
fun memberGet(vararg values: String): String = values.get(0)
fun mutableSet(values: Array<String>): String { values.set(0, "set"); return values.get(0) }
fun javaMutation(vararg values: String): String { External.replace(values); return values[0] }
fun isolation(input: Array<String>): String = javaMutation(*input) + ":" + input[0]
fun namedShares(input: Array<String>): Boolean = expose(values = input) === input
var effects = ""
fun tick(label: String, value: String): String { effects += label; return value }
fun marked(values: Array<String>): Array<String> { effects += "2"; return values }
fun ordered(input: Array<String>): String = join(tick("1", "a"), *marked(input), tick("3", "d"))
interface Bag {
    fun count(vararg values: String): Int
    fun first(vararg values: String): String = values[0]
}
class Holder(val prefix: Int) : Bag {
    override fun count(vararg values: String): Int = prefix + values.size
}
object Named { fun count(vararg values: Item): Int = values.size }
class SourceGeneric { fun <T> first(vararg values: T): T = values[0] }
class Sink<in T> { fun count(vararg values: T): Int = values.size }
