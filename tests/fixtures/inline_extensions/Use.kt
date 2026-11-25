package usage

import extensions.measure
import extensions.measureLong
import extensions.transform
import extensions.simple
import extensions.forward
import extensions.fallback
import extensions.retained
import extensions.accepts

class Effects {
    var calls: Int = 0
    fun next(): String { calls = calls + 1; return "text" }
}

fun use(effects: Effects): Int = effects.next().measure { it + 5 }
fun wide(): Long = 10000000000L.measureLong { it + 7L }
fun negate(value: Double): Double = value.transform { -it }
fun scalar(): Int = "abc".simple()
fun nested(): Int = "abcd".forward(2)
fun nullable(value: String?): Int = value.fallback { 17 }
fun retained(): Long = 12L.retained { it + 3L }
fun escape(): Int { "abc".measure { return it + 7 }; return 0 }
fun captures(seed: Int): Int = "abc".measure { it + seed }
fun reified(value: Any?): Boolean = "x".accepts<String>(value)
