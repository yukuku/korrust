package invalid
import inference.dependency.marker
fun bad(value: String?): Int {
    if (marker(value) != null) return value.length
    return 0
}
