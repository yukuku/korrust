import Lists
fun readonlyOut(value: List<out String>): Int = Lists.accept(value)
fun readonlyView(value: MutableList<out String>): Int {
    val view: List<out String> = value
    return Lists.accept(view)
}
fun identityOut(value: List<out String>) = value
class Holder {
    fun accept(value: List<out String>): Int = Lists.accept(value)
}
typealias Names = List<out String>
fun alias(value: Names): Int = Lists.accept(value)
