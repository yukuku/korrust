@JvmInline value class V(val raw: Int)
fun <T: V> identity(value:T):T = value
