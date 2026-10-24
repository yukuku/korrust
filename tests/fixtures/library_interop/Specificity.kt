package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun specificityProbe(): String = JavaApi.choose("text")
fun nullSpecificityProbe(): String = JavaApi.choose(null)
fun preferNullable(value: String?, unused: Int = 0): Int = 1
fun preferNullable(value: Any?): Int = 2
fun nullDefaultSpecificityProbe(): Int = preferNullable(null)
fun <T> preferConcrete(value: T): Int = 1
fun preferConcrete(value: CharSequence): Int = 2
fun genericSpecificityProbe(): Int = preferConcrete("text")
fun preferSubtype(value: CharSequence): Int = 1
fun preferSubtype(value: Any): Int = 2
fun subtypeSpecificityProbe(): Int = preferSubtype("text")
