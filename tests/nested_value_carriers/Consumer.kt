package consumers

import dep.Bits
import dep.Token
import dep.Packed
import dep.echo
import dep.optional

fun payload(n: Long): Long = echo(Token(Bits(n))).bits.raw
fun absent(): Boolean = optional(null) == null
fun unsignedValue(): Boolean = Packed(0xffffffffffffffffuL).unsigned == 0xffffffffffffffffuL
fun deep(n: Long): Long = dep.Deep(Token(Bits(n))).token.bits.raw
fun present(n: Long): Long = optional(Token(Bits(n)))!!.bits.raw
