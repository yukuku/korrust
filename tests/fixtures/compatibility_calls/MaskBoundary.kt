package compatibility.calls
import compatibility.calls.lib.wide
fun maskBoundary(): String = "${wide()}:${wide(p31 = 100, p32 = 200)}:${wide(p0 = 9, p33 = 300)}"
