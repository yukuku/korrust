package innerprobe
class Initialization(var value: Int) {
    inner class Child(val delta: Int) : JavaParent(value) {
        val initialized: Int = read()
        override fun read(): Int = this@Initialization.value + delta
    }
    fun make(delta: Int): Child = Child(delta)
}
class Shadow(val value: Int) {
    inner class Item(val value: Int) {
        fun sum(): Int = value + this@Shadow.value
        fun local(value: Int): Int = value + this.value + this@Shadow.value
    }
}
class PairOuter(val start: Int) {
    inner class Pair(val first: Int, val second: Int) {
        fun sum(): Int = start + first + second
    }
}
var order: Int = 0
fun record(value: Int): Int { order = order * 10 + value; return value }
fun factory(): PairOuter { record(1); return PairOuter(100) }
fun ordered(): Int = factory().Pair(second = record(2), first = record(3)).sum()
fun nullable(): PairOuter? { record(1); return null }
fun failsBeforeArguments(): Int = nullable()!!.Pair(record(2), record(3)).sum()
fun bound(outer: PairOuter): (Int, Int) -> PairOuter.Pair = outer::Pair
fun unbound(): (PairOuter, Int, Int) -> PairOuter.Pair = PairOuter::Pair

fun safeFactory(outer: PairOuter?): PairOuter.Pair? = outer?.Pair(record(2), record(3))
class ReferenceOuter(val label: String?) {
    inner class Entry(val name: String, val count: Long, val weight: Double, val maybe: String?) {
        fun text(): String = (this@ReferenceOuter.label ?: "none") + ":" + name + ":" + (maybe ?: "null")
        fun score(): Double = count.toDouble() + weight
    }
}
