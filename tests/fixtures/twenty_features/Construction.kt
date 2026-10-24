package twenty
import twenty.dep.Trace
fun implicitConstruct(): Int = Implicit().value()
fun namedConstruct(): DefaultCtor = DefaultCtor(c = Trace.mark("C", 9), a = Trace.mark("A", 4))
fun explicitConstruct(): Captured = Captured(wide = Trace.wide("W", 20L), seed = Trace.mark("S", 2))
fun importedMemberDefault(): String = DefaultMethods().compute(b = Trace.mark("B", 8))
