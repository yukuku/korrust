package invalid
import inference.dependency.*
fun bad(value: String?): Int {
    if (nullableFalse(value) != true) return value.length
    return 0
}
