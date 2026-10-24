package interop.probes
import interop.lib.*
import interop.dep.JavaApi
import kotlin.text.repeat

fun extensionProbe(): Int = "abcd".scaled(3)
fun stdlibRepeatProbe(value: String, count: Int): String = value.repeat(count)
