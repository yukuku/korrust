package binarydefaults.library
@JvmInline value class Key(val raw: Long) {
    fun plus(other: Key = Key(7L)): Key = Key(raw + other.raw)
}
@JvmInline value class Label(val raw: String)
@JvmInline value class Small(val raw: Int)
private var events: String = ""
fun reset() { events = "" }
fun trace(): String = events
fun keyEvent(tag: String): Key { events += tag; return Key(5L) }
fun textEvent(tag: String): String { events += tag; return tag }
fun top(key: Key = Key(11L), label: Label = Label("default")): Key = Key(key.raw + label.raw.length)
fun Key.mix(other: Key = Key(13L)): Key = Key(raw + other.raw)
fun reference(label: Label = Label("reference")): Label = label
fun nullable(key: Key? = null): Boolean = key == null
class Reader {
    fun read(key: Key = Key(17L), label: String = "member"): Key = Key(key.raw + label.length)
    fun ordered(key: Key = keyEvent("K"), text: String = textEvent("T")): String = trace() + ":" + text + ":" + key.raw
    fun <T> generic(key: Key = Key(19L), item: T): T = if (key.raw == 19L) item else throw IllegalStateException("wrong default")
}

fun wide(p0: Key = Key(101L), p1: Int = 1, p2: Int = 2, p3: Int = 3, p4: Int = 4, p5: Int = 5, p6: Int = 6, p7: Int = 7, p8: Int = 8, p9: Int = 9, p10: Int = 10, p11: Int = 11, p12: Int = 12, p13: Int = 13, p14: Int = 14, p15: Int = 15, p16: Int = 16, p17: Int = 17, p18: Int = 18, p19: Int = 19, p20: Int = 20, p21: Int = 21, p22: Int = 22, p23: Int = 23, p24: Int = 24, p25: Int = 25, p26: Int = 26, p27: Int = 27, p28: Int = 28, p29: Int = 29, p30: Int = 30, p31: Int = 31, p32: Int = 32, p33: Small = Small(33)): Long = p0.raw + p32 + p33.raw
@JvmInline value class OtherKey(val raw: Long)
fun readerEvent(): Reader { events += "R"; return Reader() }
fun Key?.nullableMix(other: Key = Key(7L)): Long = (this?.raw ?: 0L) + other.raw
