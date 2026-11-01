package directcontext
@JvmInline value class V(val raw: Int)
interface I { fun body(): Int { val value = V(19); return value.raw } }
class Impl : I
