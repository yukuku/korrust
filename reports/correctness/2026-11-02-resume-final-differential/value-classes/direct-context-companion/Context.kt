package directcontext
@JvmInline value class V(val raw: Int)
class Host { companion object { fun body(): Int { val value = V(11); return value.raw } } }
