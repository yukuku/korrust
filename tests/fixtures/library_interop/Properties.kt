package interop.probes
import interop.lib.*
import interop.dep.JavaApi
import interop.visibility.libraryRestricted

fun propertyProbe(): Int { libraryMutable = 8; return libraryNumber + libraryMutable }
fun restrictedGetterProbe(): Int = libraryRestricted
