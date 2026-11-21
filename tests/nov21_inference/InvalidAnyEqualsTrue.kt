package invalid
import inference.dependency.anyTrue
fun bad(value: String?): Int {
    if (anyTrue(value) == true) return value.length
    return 0
}
