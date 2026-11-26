package library
class Scope {
    fun <T> first(values: List<T>): T = values[0]
    inline fun <reified T: Any> cast(value: Any): T = value as T
}
fun inScope(block: Scope.() -> String): String = Scope().block()
fun combine(text: String, number: Int): String = text + number
