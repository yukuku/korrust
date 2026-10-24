package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun javaGenericProbe(): String = JavaApi.first<String>(listOf<String>("signature", "other"))
