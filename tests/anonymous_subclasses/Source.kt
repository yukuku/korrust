package anonymousclasses
fun create(start: Int): Base {
    var state = start
    return object : Base(start + 1), Left, Right, Marker {
        override fun compute(value: Int): Int { state += value; return super.compute(state) }
        override fun left(): Int = state
        override fun right(): Int = this.compute(2)
    }
}
fun empty(seed: Int): Base = object : Base(seed) {}
fun marker(): Marker = object : Marker {}
class Outer(val seed: Int) {
    fun make(delta: Int): Base = object : Base(seed + delta) {
        override fun compute(value: Int): Int = super.compute(value) + this@Outer.seed + delta
    }
}
fun argumentCapture(): Base {
    var state = 1
    fun next(): Int { state += 1; return state }
    return object : Base(next()) { override fun compute(value: Int): Int = state + value }
}
fun orderedArguments(): PairBase {
    var state = 0
    fun next(): Int { state += 1; return state }
    return object : PairBase(next(), next()) { override fun state(): Int = state }
}
