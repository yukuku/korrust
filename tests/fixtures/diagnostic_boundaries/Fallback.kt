import boundarylib.packet
import fallback.renamedRead
fun fallbackProbe(): String = packet()!!.renamedRead()
