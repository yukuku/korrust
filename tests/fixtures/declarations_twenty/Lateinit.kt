package declarations.probes
import declarations.support.Trace
class LateBox { lateinit var text: String; fun length(): Int = text.length }
fun lateValue(): Int { val box = LateBox(); box.text = "ready"; return box.length() }
