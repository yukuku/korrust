package next10.shorts
import next10.external.libraryPack
import next10.external.libraryScalar

fun shortSum(a: Short, b: Short): Int = a + b
fun shortUnary(a: Short): Int = -a
fun byteUnary(a: Byte): Int = +a
fun mixed(a: Int, b: Long): Long = a + b
fun reverse(a: Long, b: Int): Long = a - b
fun product(a: Short, b: Long): Long = a * b
fun quotient(a: Long, b: Short): Long = a / b
fun remainder(a: Short, b: Int): Int = a % b
fun shortOrder(a: Short, b: Int): Boolean = a < b
fun longOrder(a: Int, b: Long): Boolean = a >= b
fun byteSum(a: Byte, b: Short): Int = a + b
fun floating(a: Short, b: Double): Double = a + b
fun narrow(a: Long): Short = a.toShort()
fun byteNarrow(a: Short): Byte = a.toByte()
fun charNarrow(a: Short): Char = a.toInt().toChar()
fun widen(a: Short): Long = a.toLong()
fun boxed(a: Short): Short? = a
fun unbox(a: Short?): Short = a!!
fun contextual(): Short = -32768
fun contextualNullable(): Short? = 32767
fun text(a: Short): String = "v=$a"
fun equality(a: Short, b: Short): Boolean = a == b
fun fromInt(a: Int): Short = a.toShort()

fun empty(size: Int): ShortArray = ShortArray(size)
fun initialized(size: Int): ShortArray = ShortArray(size) { (it * 32767).toShort() }
fun get(values: ShortArray, index: Int): Short = values[index]
fun set(values: ShortArray, index: Int, value: Short): Short {
    values[index] = value
    return values[index]
}
fun literalStore(values: ShortArray) { values[0] = -32768; values[1] = 32767 }
fun total(values: ShortArray): Long {
    var total = 0L
    for (value in values) { total = total + value }
    return total
}
fun pack(vararg values: Short): ShortArray = values
fun spread(values: ShortArray): ShortArray = pack(-32768, *values, 32767)
fun factory(): ShortArray = shortArrayOf(-32768, 0, 32767)
fun index(values: ShortArray): Short = values[Effects.index()]
fun ordered(values: ShortArray): ShortArray = pack(Effects.value(1), *Effects.array(values), Effects.value(2))
fun initializeEffects(size: Int): ShortArray = ShortArray(size) { Effects.value(it) }
fun javaArray(values: ShortArray): ShortArray = Effects.array(values)
class Holder(val scalar: Short, val values: ShortArray)
fun imported(values: ShortArray): ShortArray = libraryPack(-1, *values)
fun importedScalar(value: Short): Short = libraryScalar(value)
fun nullable(values: ShortArray?): ShortArray? = values
fun cast(values: Any): ShortArray = values as ShortArray
