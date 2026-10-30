import kotlin.with as around
class LabelValue(val amount: Int)
fun aliased(): Int = around(LabelValue(3)) { this@around.amount }
fun named(): Int = around(receiver = LabelValue(4), block = { this@around.amount })
@JvmOverloads fun String.overloaded(value: Int = 1): Int = value
