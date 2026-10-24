package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun lambdaInferenceProbe(): Int = applyValue(5) { it + 3 }
