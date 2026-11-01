package boundarylib
@JvmInline value class Packet<out T>(val raw: Any?)
class Crate<T>(val raw: T)
@JvmInline value class Ticket(val n: Int)
fun <T> Packet<T>.unpack(tag: String): T = raw as T
fun <T> Crate<T>.unpack(tag: String): T = raw
fun Ticket.doubleIt(): Int = n * 2
fun packet(): Packet<String>? = Packet<String>("payload")
