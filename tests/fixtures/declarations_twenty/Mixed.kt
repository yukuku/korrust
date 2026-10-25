package declarations.probes
import declarations.support.Trace
fun before(): Int = MixedBox(2).value + MixedObject.value + after()
class MixedBox(val value: Int)
val mixedTop = 3
object MixedObject { val value = 4 }
fun after(): Int = mixedTop
