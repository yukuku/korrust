@file:Suppress("INVISIBLE_REFERENCE", "INVISIBLE_MEMBER")
package externalvalue

class Token

@PublishedApi
internal fun foldBits(first: Int, second: Int): Int = first * 31 + second

@JvmInline
value class Counter(val raw: Int) {
    @kotlin.internal.InlineOnly
    inline fun scramble(other: Counter): Counter = Counter(foldBits(raw, other.raw))
    @kotlin.internal.InlineOnly
    inline fun arrayKind(): Class<*> = Array<Token>::class.java
}
