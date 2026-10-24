package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun metadataProbe(): String = identity<String>("metadata")
