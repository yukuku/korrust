package invalid
import inference.dependency.exists
fun bad(value: String?): Int {
    if (exists(value) || true) return value.length
    return 0
}
