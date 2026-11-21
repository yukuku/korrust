package invalid
import inference.dependency.nullableTruth
fun bad(value: String?): Int {
    if (false != nullableTruth(value)) return value.length
    return 0
}
