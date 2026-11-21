package invalid
import inference.dependency.anyFalse
fun bad(value: String?): Int {
    if (anyFalse(value) == false) return value.length
    return 0
}
