package twenty
import twenty.dep.Trace
fun defaults(a: Int = Trace.mark("a", 2), b: Long = Trace.wide("b", 3L), c: Int = Trace.mark("c", a + 1)): String = "$a:$b:$c"
fun namedDefaults(): String = defaults(c = Trace.mark("C", 9), a = Trace.mark("A", 4))
fun plainDefault(value: Int = 7): Int = value
private fun privateDefault(value: Int = 8): Int = value
fun invokePrivateDefault(): Int = privateDefault()
fun priorDefault(a: Int, b: Int = a + 1): Int = b
