package anonymousclasses
open class Base(val seed: Int) {
    open fun compute(value: Int): Int = seed + value
    fun inherited(): Int = seed
}
interface Left { fun left(): Int }
interface Right { fun right(): Int }
interface Marker
open class PairBase(val first: Int, val second: Int) { open fun state(): Int = 0 }
