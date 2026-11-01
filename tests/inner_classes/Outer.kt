package innerprobe
class Outer(var value: Int) {
    private var secret: Int = 7
    private fun hidden(delta: Int): Int { secret = secret + delta; return secret }
    fun make(delta: Int): Inner = Inner(delta)
    inner class Inner(val delta: Int) {
        val atInit: Int = value + delta
        fun read(): Int = value + delta
        fun update(next: Int): Int { value = next; return this@Outer.value }
        fun reveal(): Int = hidden(delta)
        fun readSecret(): Int = secret
        fun callback(): () -> Int = { value + delta }
        fun outer(): Outer = this@Outer
    }
}
fun qualified(outer: Outer, delta: Int): Outer.Inner = outer.Inner(delta)

class GenericArgumentsOuter {
    inner class Entry(val values: List<String>)
}
