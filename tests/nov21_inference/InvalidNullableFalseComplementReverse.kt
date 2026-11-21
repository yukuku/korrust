package invalid
import inference.dependency.nullableFalse
fun bad(value: String?): Int {
    if (true != nullableFalse(value)) return value.length
    return 0
}
