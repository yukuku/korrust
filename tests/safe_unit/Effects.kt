class Effects {
    var calls: Int = 0
    fun mark(value: Int) { calls = calls * 10 + value }
}
var receivers: Int = 0
var arguments: Int = 0
fun receiver(value: Effects?): Effects? { receivers++; return value }
fun argument(): Int { arguments++; return 7 }
fun reset() { receivers = 0; arguments = 0 }
fun safe(value: Effects?): Unit? = receiver(value)?.mark(argument())
fun statement(value: Effects?) { receiver(value)?.mark(argument()) }
fun nullableIdentity(value: Unit?): Unit? = value
fun widened(value: Effects?): Any? = receiver(value)?.mark(argument())
fun elvis(value: Effects?, other: Effects) { receiver(value)?.mark(argument()) ?: other.mark(3) }
fun asserted(value: Effects?) { receiver(value)?.mark(argument())!! }
fun callback(value: Effects?): () -> Unit? = { receiver(value)?.mark(argument()) }
fun <T> identity(value: T): T = value
fun generic(value: Effects?): Unit? = identity(receiver(value)?.mark(argument()))
fun force(value: Unit?): Unit = value!!
class UnitHolder(val value: Unit?)
fun external(value: External?): Unit? = value?.touch(argument())
