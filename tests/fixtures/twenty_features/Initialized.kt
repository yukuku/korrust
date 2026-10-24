package twenty
import twenty.dep.Trace
class Initialized(val seed: Int) {
    var count: Int = Trace.mark("1", seed)
    val next: Int = Trace.mark("2", count + 1)
    val label: String = Trace.text("3", "ready")
    fun change(value: Int): Int { count = value; return next }
}
