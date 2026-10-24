package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun samProbe(base: Int): Int = JavaApi.apply({ value -> value + base }, 5)
