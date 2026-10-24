package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun invocationProbe(block: (Int) -> Int): Int = block(9)
