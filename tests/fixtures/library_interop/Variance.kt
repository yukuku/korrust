package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun varianceProbe(value: List<out String>): String = JavaApi.projected<String>(value)
