package compatibility.calls.invalid
import compatibility.calls.lib.CallEffects
fun bad(flag: Boolean, value: String?): String = if (flag) CallEffects.platform("P", true) else value
