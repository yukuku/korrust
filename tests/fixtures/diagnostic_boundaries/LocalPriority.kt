package localcase
import boundarylib.packet
import higher.localRead
fun localPriorityProbe(): String = packet()!!.localRead()
