package updates
object Events {
    var trace: String = ""
    var failure: Int = 0
    fun record(part: String) { trace = trace + part }
    fun rhs(): Int { record("r"); if (failure == 2) throw IllegalStateException("rhs"); return 2 }
    fun index(): Int { record("i"); if (failure == 5) throw IllegalStateException("index"); return 0 }
    fun wide(): Long { record("j"); if (failure == 6) throw IllegalStateException("wide"); return 7L }
}
class Counter(val value: Int) {
    operator fun inc(): Counter { Events.record("+"); if (Events.failure == 3) throw IllegalStateException("op"); return Counter(value + 1) }
    operator fun dec(): Counter { Events.record("-"); return Counter(value - 1) }
    operator fun plus(delta: Int): Counter { Events.record("p"); if (Events.failure == 3) throw IllegalStateException("op"); return Counter(value + delta) }
}
class Mutating(var value: Int) {
    operator fun plusAssign(delta: Int) { Events.record("a"); if (Events.failure == 3) throw IllegalStateException("op"); value = value + delta }
}
class Holder(initial: Counter) {
    var saved: Counter = initial
    var item: Counter
        get() { Events.record("g"); if (Events.failure == 1) throw IllegalStateException("get"); return saved }
        set(value) { Events.record("s"); if (Events.failure == 4) throw IllegalStateException("set"); saved = value }
}
class ReadOnly(val item: Mutating)
class Indexed(initial: Counter) {
    var saved: Counter = initial
    operator fun get(index: Int, wide: Long): Counter { Events.record("g"); if (Events.failure == 1) throw IllegalStateException("get"); return saved }
    operator fun set(index: Int, wide: Long, value: Counter) { Events.record("s"); if (Events.failure == 4) throw IllegalStateException("set"); saved = value }
}
fun holder(value: Holder): Holder { Events.record("h"); if (Events.failure == 7) throw IllegalStateException("receiver"); return value }
fun indexed(value: Indexed): Indexed { Events.record("h"); return value }
fun readOnly(value: ReadOnly): ReadOnly { Events.record("h"); return value }
fun post(value: Holder): Counter = holder(value).item++
fun down(value: Holder): Counter = holder(value).item--
fun compound(value: Holder) { holder(value).item += Events.rhs() }
fun assign(value: ReadOnly) { readOnly(value).item += Events.rhs() }
fun indexPost(value: Indexed): Counter = indexed(value)[Events.index(), Events.wide()]++
fun indexCompound(value: Indexed) { indexed(value)[Events.index(), Events.wide()] += Events.rhs() }
fun arrayPost(value: IntArray): Int = value[Events.index()]++
fun arrayCompound(value: LongArray) { value[Events.index()] += Events.rhs().toLong() }
fun arrayCounter(value: Array<Counter>): Counter = ++value[Events.index()]
fun local(): Int { var value = Counter(2); val first = value++; val second = ++value; value--; return first.value * 100 + second.value * 10 + value.value }
fun captured(): () -> Int { var value = Counter(2); return { val before = value++; before.value * 10 + value.value } }
open class Base(val value: Int) {
    operator fun inc(): Child = Child(value + 1)
}
class Child(value: Int): Base(value)
fun subtype(): Int { var value: Base = Base(3); val next = ++value; return next.value + value.value }
class Extension(val value: Int)
operator fun Extension.inc(): Extension = Extension(value + 1)
operator fun Extension.dec(): Extension = Extension(value - 1)
fun extension(): Int { var value = Extension(2); val before = value++; val after = --value; return before.value * 10 + after.value }
fun inherited(): Int { var value = Child(4); value++; return value.value }
interface Tick { operator fun inc(): Tick; fun number(): Int }
class Ticker(val value: Int): Tick {
    override fun inc(): Tick = Ticker(value + 1)
    override fun number(): Int = value
}
fun dispatch(): Int { var value: Tick = Ticker(7); val old = value++; return old.number() * 10 + value.number() }
fun rebind(initial: Holder): Int {
    var current = initial
    fun rhs(): Int { Events.record("r"); current = Holder(Counter(100)); return 2 }
    current.item += rhs()
    return current.saved.value * 1000 + initial.saved.value
}
fun numbers(): Long {
    var byte: Byte = 127
    var short: Short = 32767
    var char: Char = '\uffff'
    var long: Long = 10L
    var float: Float = 2.0f
    var double: Double = 3.0
    byte++; ++short; char++; ++long; float--; --double
    return byte.toLong() + short.toLong() + char.toInt().toLong() + long + float.toLong() + double.toLong()
}
class Dual(var value: Int) {
    operator fun plusAssign(delta: Int) { Events.record("a"); value = value + delta }
    operator fun plus(delta: Int): Dual { Events.record("p"); return Dual(value + delta) }
}
class DualHolder(val item: Dual)
class DualIndex(val item: Dual) { operator fun get(index: Long): Dual { Events.record("g"); return item } }
fun dualProperty(value: DualHolder) { value.item += Events.rhs() }
fun dualIndex(value: DualIndex) { value[Events.wide()] += Events.rhs() }
class NarrowIndex(var item: Counter) {
    operator fun get(index: Byte): Counter = item
    operator fun set(index: Byte, value: Counter) { item = value }
}
fun narrow(value: NarrowIndex): Int { val index: Byte = 1; val old = value[index]++; return old.value * 10 + value.item.value }
class Numeric(initial: Int) {
    var saved: Int = initial
    var item: Int
        get() { Events.record("g"); return saved }
        set(value) { Events.record("s"); saved = value }
}
fun primitiveProperty(value: Numeric) { value.item *= Events.rhs() }
fun primitivePost(value: Numeric): Int = value.item++
