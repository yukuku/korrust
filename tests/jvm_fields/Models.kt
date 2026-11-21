package fieldapi
import kotlin.jvm.JvmField as Exposed
class Capsule(@Exposed val name: String, @field:Exposed var count: Int, val ordinary: String) {
    @Exposed val derived: String = name + "!"
    @Exposed var label: String = name
    @Exposed val absent: String? = null
    @Exposed val maybe: Int? = null
    @Exposed val isReady: Boolean = true
    @Exposed var ticks: Long = 7L
    fun increment(): Int {
        count += 1
        return count
    }
}
private var events: String = ""
private fun mark(value: String): String {
    events += value
    return value
}
class Ordered(@Exposed val left: String) {
    @Exposed val next: String = mark(left + "b")
    init { mark("i") }
    @Exposed val last: String = mark("c")
}
fun ordered(): String {
    events = ""
    val value = Ordered(mark("a"))
    return events + ":" + value.left + value.next + value.last
}
fun ordinary(): String {
    val value = Capsule("name", 2, "normal")
    value.count = 4
    value.label = "changed"
    return value.name + value.count + value.label + value.ordinary
}
class AccessorName(@Exposed val value: String) {
    fun getValue(): String = "explicit"
}
class Legacy {
    lateinit var text: String
}
class GenericBox<T>(@Exposed val item: T)
open class Protected {
    @Exposed protected val value: String = "protected"
}
class Guessed {
    @Exposed val text = "inferred"
}
fun guessed(): String = Guessed().text
