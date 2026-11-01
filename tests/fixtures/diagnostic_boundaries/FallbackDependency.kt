package fallback
fun <T> boundarylib.Packet<T>.renamedRead(): String = "packet"
fun Any.renamedRead(): String = "any"
fun <T> boundarylib.Packet<T>.notInfix(other: Int): String = "plain"
