package consumer
import roots.*
fun describe(v: Any): String = describeArbitrary(v)
fun fingerprint(v: Any): Int = fingerprintArbitrary(v)
fun equivalent(v: Any, w: Any?): Boolean = equivalentArbitrary(v, w)
fun inherited(v: Leaf): Int = readInherited(v) + readEpoch(v)
fun ref(v: Any?): String = "pre:$v:post"
fun chars(v: CharArray): String = "$v"
fun ints(v: IntArray): String = "$v"
fun array(v: Array<String>): String = "$v"
fun evaluate(): String = "${Effects.next()}/${Effects.next()}"
fun fails(): String = "${Effects.broken()}/${Effects.next()}"
fun nullable(v: Leaf?): String = "[$v]"
fun primitive(v: Int): String = "[$v]"
fun vacantStrings(n: Int): Array<String?> = makeVacancies<String>(n)
fun vacantInts(n: Int): Array<Int?> = makeVacancies<Int>(n)
fun vacantLongs(n: Int): Array<Long?> = makeVacancies<Long>(n)
fun vacantArrays(n: Int): Array<IntArray?> = makeVacancies<IntArray>(n)
fun vacantNested(n: Int): Array<Array<String>?> = makeVacancies<Array<String>>(n)
fun vacantLeaf(n: Int): Array<Leaf?> = makeVacancies<Leaf>(n)
fun gathered(v: List<String>): Array<String> = gatherItems<String>(v)
fun directGathered(v: List<String>): Array<String> = v.toTypedArray()
fun vacantBooleans(n: Int): Array<Boolean?> = makeVacancies<Boolean>(n)
fun vacantChars(n: Int): Array<Char?> = makeVacancies<Char>(n)
fun vacantBytes(n: Int): Array<Byte?> = makeVacancies<Byte>(n)
fun vacantShorts(n: Int): Array<Short?> = makeVacancies<Short>(n)
fun vacantFloats(n: Int): Array<Float?> = makeVacancies<Float>(n)
fun vacantDoubles(n: Int): Array<Double?> = makeVacancies<Double>(n)
fun vacantNullable(n: Int): Array<String?> = makeVacancies<String?>(n)
fun vacancyEffect(): Array<String?> = makeVacancies<String>(Effects.size())
fun vacancyThrow(): Array<String?> = makeVacancies<String>(Effects.failedSize())
@JvmInline
value class Label(val text: String) {
    override fun toString(): String = "label:$text"
}
fun valueClassTemplate(v: String): String = "${Label(v)}"
fun vacantLabels(n: Int): Array<Label?> = makeVacancies<Label>(n)

fun hiddenParent(v: Visible): Int = hiddenBase(v)
fun classTemplate(): String = "${String::class}"
fun unitObjectTemplate(): String = "${Unit}"
fun <T> genericTemplate(value: T): String = "$value"
fun listTemplate(value: List<String>): String = "$value"
