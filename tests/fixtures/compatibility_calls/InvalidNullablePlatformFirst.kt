package compatibility.calls.invalid
import compatibility.calls.lib.CallEffects
fun bad(flag: Boolean, value: String?): String = if (flag) value else CallEffects.platform("P", true)
