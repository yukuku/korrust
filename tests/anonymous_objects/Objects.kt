package anonymousprobe
interface Counter {
    fun add(delta: Int): Int
    fun current(): Int
    fun self(): Counter
    fun doubled(): Int = current() * 2
}
fun counter(base: Int): Counter {
    var state = base
    return object : Counter {
        override fun add(delta: Int): Int { state += delta; return state }
        override fun current(): Int = state
        override fun self(): Counter = this
    }
}
fun shared(): Int {
    var state = 2
    val value: Counter = object : Counter {
        override fun add(delta: Int): Int { state += delta; return state }
        override fun current(): Int = state
        override fun self(): Counter = this
    }
    state = 7
    val read = value.current()
    value.add(3)
    return read * 100 + state
}
class Owner(val base: Int) {
    fun make(extra: Int): Counter = object : Counter {
        override fun add(delta: Int): Int = base + extra + delta
        override fun current(): Int = this@Owner.base + extra
        override fun self(): Counter = this
    }
}
fun immutable(base: Int): Counter {
    val twice = base * 2
    return object : Counter {
        override fun add(delta: Int): Int = twice + delta
        override fun current(): Int = twice
        override fun self(): Counter = this
    }
}
fun zero(): Counter = object : Counter {
    override fun add(delta: Int): Int = delta
    override fun current(): Int = 0
    override fun self(): Counter = this
}
fun nested(): Int {
    var state = 1
    val make: () -> Counter = {
        object : Counter {
            override fun add(delta: Int): Int { state += delta; return state }
            override fun current(): Int = state
            override fun self(): Counter = this
        }
    }
    val value = make()
    state = 9
    value.add(4)
    return state
}
fun siblings(): Int {
    var state = 1
    val setter: (Int) -> Unit = { state = it }
    val value: Counter = object : Counter {
        override fun add(delta: Int): Int { state += delta; return state }
        override fun current(): Int = state
        override fun self(): Counter = this
    }
    setter(7)
    val before = value.current()
    value.add(1)
    return before * 10 + state
}
fun shadow(current: Int): Counter = object : Counter {
    override fun add(current: Int): Int = current
    override fun current(): Int { val current = 4; return current }
    override fun self(): Counter = this
}
fun effects(): Int {
    var calls = 0
    val tick: () -> Int = { calls++; calls }
    val seed = tick()
    val value: Counter = object : Counter {
        override fun add(delta: Int): Int = tick() + seed + delta
        override fun current(): Int = calls
        override fun self(): Counter = this
    }
    val before = calls
    val result = value.add(4)
    return before * 2000 + result
}
class FactoryOwner(private val base: Int) {
    private fun plus(delta: Int): Int = base + delta
    fun make(): Counter = object : Counter {
        override fun add(delta: Int): Int = plus(delta)
        override fun current(): Int = plus(5)
        override fun self(): Counter = this
    }
}
interface Wide { fun read(delta: Long): Long }
fun wide(base: Long, extra: Double, text: String): Wide = object : Wide {
    override fun read(delta: Long): Long = base + delta + extra.toLong() + text.length.toLong() - 1L
}
class CollisionOwner(val base: Int) {
    fun make(): Counter {
        val `$outer$0` = 6
        return object : Counter {
            override fun add(delta: Int): Int = `$outer$0` + base + delta
            override fun current(): Int = `$outer$0` + this@CollisionOwner.base
            override fun self(): Counter = this
        }
    }
}
class PrecedenceOwner(val base: Int) {
    fun current(): Int = 100
    fun make(): Counter = object : Counter {
        override fun add(delta: Int): Int = current() + delta
        override fun current(): Int = base
        override fun self(): Counter = this
    }
}
fun mixedShadow(base: Int): Counter = object : Counter {
    override fun add(base: Int): Int = base
    override fun current(): Int = base
    override fun self(): Counter = this
}
fun mixedLocal(base: Int): Counter = object : Counter {
    override fun add(delta: Int): Int { val base = 4; return base + delta }
    override fun current(): Int = base
    override fun self(): Counter = this
}
fun namedValue(): Counter {
    val current = 7
    return object : Counter {
        override fun add(delta: Int): Int = current + delta + current()
        override fun current(): Int = 2
        override fun self(): Counter = this
    }
}
fun namedCallable(): Counter {
    val current: () -> Int = { 7 }
    return object : Counter {
        override fun add(delta: Int): Int = current() + delta
        override fun current(): Int = 2
        override fun self(): Counter = this
    }
}
fun lambdaShadow(base: Int): Int {
    val value: () -> Int = { val before = base; val base = 4; before + base }
    return value()
}
