package anonymousmembers
interface Counted {
    val count: Int
    var label: String
    fun add(value: Int): Int
}
fun stored(seed: Int): Counted {
    return object : Counted {
        override var count = seed
        override var label = "initial"
        override fun add(value: Int): Int { count += value; return count }
    }
}
fun computed(seed: Int): Counted = object : Counted {
    private var state = seed
    override val count: Int get() = state
    override var label: String = "label"
        get() = field + state
        set(value) { field = value }
    override fun add(value: Int): Int { state += value; return count }
}
fun localMembers(seed: Int): Int {
    val value = object : Counted {
        var own = seed
        override val count: Int get() = own
        override var label = "local"
        fun multiply(value: Int): Int = own * value
        override fun add(value: Int): Int { own += value; return own }
    }
    value.own = value.multiply(3)
    return value.add(2) * 100 + value.own
}
var effects = ""
fun tick(text: String, value: Int): Int { effects += text; return value }
fun ordered(): Counted = object : Counted {
    val first = tick("a", 1)
    override var count = tick("b", first + 1)
    override var label = "ordered"
    override fun add(value: Int): Int { count += value; return count }
}
open class Parent(val base: Int) {
    init { effects += "p" }
}
fun inherited(seed: Int): Counted = object : Parent(tick("s", seed)), Counted {
    override val count = tick("i", base)
    override var label = "inherited"
    override fun add(value: Int): Int = count + value
}
class Outer(val count: Int) {
    fun make(): Counted = object : Counted {
        override var count = 4
        override var label = "outer"
        override fun add(value: Int): Int = count + this@Outer.count + value
    }
}
