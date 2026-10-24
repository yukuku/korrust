package twenty
import twenty.dep.Trace
class Captured(seed: Int, wide: Long) {
    val first: Int = Trace.mark("A", seed)
    val second: Long = Trace.wide("B", wide)
    val third: Int = Trace.mark("C", first + 1)
    fun total(): String = "$second:$third"
}
