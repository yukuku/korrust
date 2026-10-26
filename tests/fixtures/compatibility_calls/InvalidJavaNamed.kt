package compatibility.calls.invalid
import compatibility.calls.lib.CallEffects
fun bad(): Int = CallEffects.choose(value = 4)
