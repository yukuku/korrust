package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun boundProbe(): Int = bounded<String>("bound") + JavaApi.bounded<String>("java")
