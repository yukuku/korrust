package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun platformProbe(nil: Boolean): String = JavaApi.platform(nil)
fun nullableProbe(): String? = JavaApi.nullable()
fun annotatedProbe(): Int = JavaApi.nonnull().length
