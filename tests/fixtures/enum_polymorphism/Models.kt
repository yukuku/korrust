package enums.polymorphism

object Trace {
    var log: String = ""
    fun mark(value: String, result: Int): Int {
        log += value
        return result
    }
}

interface Action {
    fun run(input: Int): String
    val caption: String
}

enum class Task(val first: Int = Trace.mark("d", 3), val second: Int = first + 2) : Action {
    LEFT(second = Trace.mark("s", 8), first = Trace.mark("f", 4)) {
        val initialized: Int = Trace.mark("l", 9)
        init { Trace.mark("i", 0) }
        override fun run(input: Int): String = "L" + (first + second + input + initialized)
        override val caption: String get() = name + ":" + first + "!"
    },
    RIGHT {
        override fun run(input: Int): String = "R" + (first + second + input)
    },
    EMPTY(1, 2) {};

    open override fun run(input: Int): String = "E" + (first + second + input)
    open override val caption: String get() = name + ":" + first

    companion object {
        val marker: String = "companion"
        fun first(): Task = LEFT
    }
}

enum class AbstractAction(val number: Long, val fraction: Double) {
    A(8L, 0.5) { override fun score(): Double = number.toDouble() + fraction },
    B(10L, 2.5) { override fun score(): Double = super.adjust() + fraction };

    abstract fun score(): Double
    open fun adjust(): Double = number.toDouble()
}

enum class PerEntry : Action {
    A {
        override fun run(input: Int): String = "a" + input
        override val caption: String get() = "A"
    },
    B {
        override fun run(input: Int): String = "b" + input
        override val caption: String = "B"
    }
}

enum class DefaultOnly(val value: Int = Trace.mark("v", 7), val following: Int = value + 1) {
    A, B(11), C(following = 20)
}
