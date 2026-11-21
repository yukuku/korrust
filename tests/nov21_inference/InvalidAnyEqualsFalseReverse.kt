package invalid
import inference.dependency.anyFalse
fun bad(value: String?): Int {
    if (false == anyFalse(value)) return value.length
    return 0
}
