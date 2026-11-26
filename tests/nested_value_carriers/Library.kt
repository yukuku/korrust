package dep

@JvmInline value class Bits(val raw: Long)
@JvmInline value class Token(val bits: Bits)
@JvmInline value class Packed(val unsigned: ULong)
fun echo(token: Token): Token = token
fun optional(t: Token?): Token? = t
@JvmInline value class Hidden private constructor(val n: Long)
@JvmInline value class Deep(val token: Token)
