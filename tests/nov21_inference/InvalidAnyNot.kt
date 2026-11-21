package invalid
import inference.dependency.anyFalse
operator fun Any.not(): Boolean = true
fun bad(value: String?): Int {
    if (!anyFalse(value)) return value.length
    return 0
}
