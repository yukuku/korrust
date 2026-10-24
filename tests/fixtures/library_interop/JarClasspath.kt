package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun jarProbe(): Int = JavaApi.bounded("archive")
