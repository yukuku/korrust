import boundarylib.packet
import boundarylib.unpack
fun reveal(): String = packet()!!.unpack("tag")
