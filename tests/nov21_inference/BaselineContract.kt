package baseline
import inference.dependency.exists
fun guard(value: String?): Int {
    if (!exists(value)) return -1
    return value.length
}
