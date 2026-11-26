package nullablevalues.library
@JvmInline value class Mode(val raw: Int)
@JvmInline value class LongValue(val raw: Long)
@JvmInline value class Text(val raw: String)
@JvmInline value class NullableText(val raw: String?)
@JvmInline value class NullableNumber(val raw: Int?)
@JvmInline value class OtherMode(val raw: Int)
class Holder<T>(val value: T)
object Values {
    val mode: Mode = Mode(7)
    val nullText: NullableText = NullableText(null)
    val nullNumber: NullableNumber = NullableNumber(null)
}
fun mode(): Mode = Mode(11)
fun longValue(): LongValue = LongValue(17L)
fun text(): Text = Text("text")
fun nullableText(present: Boolean): Text? = if (present) Text("present") else null
fun nullText(): NullableText = NullableText(null)
fun nullNumber(): NullableNumber = NullableNumber(null)
fun defaultMode(value: Mode = Mode(13)): Mode = value
fun optionalMode(value: Mode? = Mode(19)): Mode? = value
fun defaultNullText(value: NullableText = NullableText(null)): NullableText = value
fun optionalNullText(value: NullableText? = NullableText(null)): NullableText? = value
fun defaultNullNumber(value: NullableNumber = NullableNumber(null)): NullableNumber = value
fun optionalNullNumber(value: NullableNumber? = NullableNumber(null)): NullableNumber? = value
fun <T> identity(value: T): T = value
fun inspectMode(value: Mode?): Int = value?.raw ?: -1
fun inspectLong(value: LongValue?): Long = value?.raw ?: -1L
fun inspectText(value: Text?): String = value?.raw ?: "absent"
fun inspectNullText(value: NullableText?): String = if (value == null) "absent" else "wrapped:" + (value.raw ?: "null")
fun inspectNullNumber(value: NullableNumber?): String = if (value == null) "absent" else "wrapped:" + (value.raw?.toString() ?: "null")
class TextHolder(val value: NullableText = NullableText(null))
class NumberHolder(val value: NullableNumber = NullableNumber(null))
fun mixed(value: NullableText, mode: Mode = Mode(31)): NullableText = if (mode.raw == 31) value else NullableText("wrong")
fun <T> genericDefault(value: T? = null): T? = value
class MixedHolder(val value: NullableText = NullableText("default"), val mode: Mode = Mode(31))
fun both(value: NullableText = NullableText("default"), mode: Mode = Mode(31)): NullableText = if (mode.raw == 31) value else NullableText("wrong")
