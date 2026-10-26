package compatibility.calls
import compatibility.calls.lib.*
fun genericConstraints(): String {
    val nullable: String? = identity(null)
    val primitive: Int = transform(7) { it + 2 }
    val reference: String = transform("a") { it + "b" }
    val result: Int? = transform(2) { if (it == 2) null else it }
    val fromLet = 3.let { if (it == 0) 4 else it }
    return "$nullable:$primitive:$reference:$result:$fromLet:${bounded("ok")}"
}
