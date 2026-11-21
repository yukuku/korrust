package invalid
import inference.dependency.exists
fun bad(value: String?): Int {
    if (exists(value) && false) return 0
    return value.length
}
