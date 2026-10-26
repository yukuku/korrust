package compatibility.calls.lib
fun <T> identity(value: T): T = value
fun <T, R> transform(value: T, block: (T) -> R): R = block(value)
fun <T: CharSequence> bounded(value: T): T = value
fun defaults(first: Int = CallEffects.mark("A", 1), middle: Int = CallEffects.mark("B", 2), last: Int = CallEffects.mark("C", 3)): Int = first * 100 + middle * 10 + last
class DefaultBox(val first: Int = CallEffects.mark("A", 1), val last: Int = CallEffects.mark("C", 3)) {
    fun combine(first: Int = CallEffects.mark("D", 4), last: Int = CallEffects.mark("E", 5)): Int = this.first * 1000 + this.last * 100 + first * 10 + last
}
fun receiver(): DefaultBox { CallEffects.mark("R", 0); return DefaultBox(1, 3) }
fun kotlinInts(vararg values: Int): Int { var total = 0; for (value in values) total += value; return total }
fun kotlinStrings(vararg values: String?): Int = values.size
fun <T> genericCount(vararg values: T): Int = values.size
fun preserve(vararg values: Int): IntArray = values
fun wide(p0: Int = 0, p1: Int = 1, p2: Int = 2, p3: Int = 3, p4: Int = 4, p5: Int = 5, p6: Int = 6, p7: Int = 7, p8: Int = 8, p9: Int = 9, p10: Int = 10, p11: Int = 11, p12: Int = 12, p13: Int = 13, p14: Int = 14, p15: Int = 15, p16: Int = 16, p17: Int = 17, p18: Int = 18, p19: Int = 19, p20: Int = 20, p21: Int = 21, p22: Int = 22, p23: Int = 23, p24: Int = 24, p25: Int = 25, p26: Int = 26, p27: Int = 27, p28: Int = 28, p29: Int = 29, p30: Int = 30, p31: Int = 31, p32: Int = 32, p33: Int = 33): Int = p0 + p31 + p32 + p33
fun strictStrings(vararg values: String): Int = values.size
fun <T: CharSequence> genericText(vararg values: T): Int = values.size
fun ambiguous(vararg values: String): Int = values.size
fun ambiguous(vararg values: Int): Int = values.size
fun nullableDefault(value: String? = null): String = value ?: "none"
fun <T> component(vararg values: T): String = values.javaClass.componentType.name
fun arrayWithEffect(vararg values: Int): IntArray {
    CallEffects.mark("Q", 0)
    if (values.isNotEmpty()) values[0] += 10
    return values
}
