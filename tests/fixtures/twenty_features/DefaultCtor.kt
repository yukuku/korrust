package twenty
import twenty.dep.Trace
class DefaultCtor(val a: Int = Trace.mark("a", 2), val b: Long = Trace.wide("b", 3L), val c: Int = Trace.mark("c", a + 1)) {
    val initialized: Int = Trace.mark("i", c)
}
