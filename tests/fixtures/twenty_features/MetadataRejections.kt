package twenty.rejected
import twenty.*
fun nonnull(value: Signatures) { value.entries(null) }
fun elementNullability(value: List<Int?>): Int = intList(value)
fun readOnly(value: RichRecord) { value.score = 2.0f }
fun nonproperty(value: Captured): Int = value.seed
fun privateField(value: MutableCtor): Long = value.hidden
fun wrongBoxed(value: Int?): Int = value
