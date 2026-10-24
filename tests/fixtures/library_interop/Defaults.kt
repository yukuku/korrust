package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun defaultProbe(): Int = defaults(b = JavaApi.mark("B", 8))
fun namedProbe(): Int = defaults(b = JavaApi.mark("B", 8), a = JavaApi.mark("A", 4))
