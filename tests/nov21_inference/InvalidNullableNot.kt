package invalid
import inference.dependency.nullableFalse
operator fun Boolean?.not(): Boolean = true
fun bad(value: String?): Int {
    if (!nullableFalse(value)) return value.length
    return 0
}
