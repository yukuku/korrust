package invalid
import inference.dependency.exists
fun bad(input: String?): Int {
    var value = input
    if (!exists(value)) return -1
    value = null
    return value.length
}
