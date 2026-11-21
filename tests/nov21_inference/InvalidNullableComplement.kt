package invalid
import inference.dependency.*
fun bad(value: String?): Int {
    if (nullableTruth(value) != false) return value.length
    return 0
}
