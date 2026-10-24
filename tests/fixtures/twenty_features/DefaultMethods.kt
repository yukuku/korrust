package twenty
import twenty.dep.Trace
class DefaultMethods {
    fun compute(a: Long = Trace.wide("a", 3L), b: Int = Trace.mark("b", 4)): String = "$a:$b"
}
