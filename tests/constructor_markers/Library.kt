package markers.library
@JvmInline value class Key(val raw: Long)
@JvmInline value class Small(val raw: Int)
class PairBox(val key: Key = Key(11L), val small: Small = Small(7), val text: String = "default")
class NullableBox(val key: Key? = null)
class GenericBox<T>(val key: Key = Key(13L), val item: T)
class Hidden private constructor(val key: Key) {
    companion object { fun make(): Hidden = Hidden(Key(1L)) }
}
class InternalBox internal constructor(val key: Key = Key(19L))

class WideBox(val p0: Key = Key(101L), val p1: Int = 1, val p2: Int = 2, val p3: Int = 3, val p4: Int = 4, val p5: Int = 5, val p6: Int = 6, val p7: Int = 7, val p8: Int = 8, val p9: Int = 9, val p10: Int = 10, val p11: Int = 11, val p12: Int = 12, val p13: Int = 13, val p14: Int = 14, val p15: Int = 15, val p16: Int = 16, val p17: Int = 17, val p18: Int = 18, val p19: Int = 19, val p20: Int = 20, val p21: Int = 21, val p22: Int = 22, val p23: Int = 23, val p24: Int = 24, val p25: Int = 25, val p26: Int = 26, val p27: Int = 27, val p28: Int = 28, val p29: Int = 29, val p30: Int = 30, val p31: Int = 31, val p32: Int = 32, val p33: Small = Small(33))
private var events: String = ""
fun reset() { events = "" }
fun trace(): String = events
fun keyEvent(tag: String): Key { events += tag; return Key(1L) }
fun smallEvent(tag: String): Small { events += tag; return Small(2) }
fun textEvent(tag: String): String { events += tag; return tag }
class Ordered(val key: Key = keyEvent("K"), val small: Small = smallEvent("S"), val text: String = textEvent("T"))
class Evolved(val key: Key = Key(59L), val suffix: String = "current") {
    @Deprecated("binary compatibility", level = DeprecationLevel.HIDDEN)
    constructor(key: Key = Key(3L)): this(key, "hidden")
}
class HiddenApi @Deprecated("binary compatibility", level = DeprecationLevel.HIDDEN) constructor(val key: Key = Key(1L))
