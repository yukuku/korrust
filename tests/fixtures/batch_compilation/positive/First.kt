package batch.sample

fun inferred() = forward() + seed
val inferredProperty = seed + 2
fun even(value: Int): Boolean = if (value == 0) true else odd(value - 1)
class Left(val right: Right?)
fun choose(value: String): Int = value.length + 10
fun combined(): Int = choose(4) + choose("ab") + weighted(identity(2)) + Holder.make(seed).value
private fun hidden(): Int = 13
fun companionValue(): Int = Holder.DEFAULT.value + Holder.DEFAULT.bonus
fun ownPrivate(): Int = hidden()
fun publicInternal(): Int = moduleOnly()
