package boundary
import inference.dependency.exists
fun mutable(input: String?): Int {
    var value = input
    if (!exists(value)) return -1
    return value.length
}
