@JvmInline value class V(val raw: Int)
class Outer { inner class Holder(val value: V) }
