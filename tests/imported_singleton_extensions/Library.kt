package singleton.library

open class Base<T> { fun T.inherited(): T = this }

object Words : Base<String>() {
    init { Events.add("O") }
    private var state = 0
    fun String.decorate(suffix: String = "!"): String { Events.add("M"); return this + suffix }
    fun Int.decorate(suffix: String): String = this.toString() + suffix
    fun <T> List<T>.head(): T = this[0]
    fun String?.orText(fallback: String): String = this ?: fallback
    val String.measure: Int get() = length + 10
    var String.score: Int
        get() = state + length
        set(value) { state = value - length }
    private fun String.secret(): String = this
}

class Host {
    companion object Named {
        init { Events.add("C") }
        fun String.decorate(suffix: String = "c"): String { Events.add("N"); return this + suffix }
    }
}

class Ordinary { fun String.decorate(): String = this }

class Pretender private constructor() {
    companion object { @JvmField val INSTANCE: Pretender = Pretender() }
    fun String.mimic(): String = this
}
