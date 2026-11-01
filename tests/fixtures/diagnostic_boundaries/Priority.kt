import boundarylib.packet
import higher.priorityRead
fun priorityProbe(): String = packet()!!.priorityRead()
