package inference
import inference.dependency.marker
fun nullResult(value: String?): Int {
    if (marker(value) == null) return value.length
    return -1
}
