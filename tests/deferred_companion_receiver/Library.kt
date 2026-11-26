package numbers
operator fun Holder.plusAssign(n: Int) { value += n }
class Holder(var value: Int)
fun runCallback(block: () -> Unit) { block() }
