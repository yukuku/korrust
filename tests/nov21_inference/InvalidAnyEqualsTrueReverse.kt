package invalid
import inference.dependency.anyTrue
fun bad(value: String?): Int {
    if (true == anyTrue(value)) return value.length
    return 0
}
