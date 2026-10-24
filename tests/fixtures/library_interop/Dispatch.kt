package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun inheritedProbe(value: BaseSource): Int = value.value()
fun interfaceProbe(value: ValueSource): Int = value.value()
