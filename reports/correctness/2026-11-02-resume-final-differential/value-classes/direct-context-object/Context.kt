package directcontext
@JvmInline value class V(val raw: Int)
object O { fun body(): Int { val value = V(7); return value.raw } }
