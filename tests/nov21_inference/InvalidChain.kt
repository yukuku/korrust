package invalid
import inference.dependency.exists
private fun Boolean.always(): Boolean = true
fun bad(value: String?): Int {
    if (exists(value).always()) return value.length
    return 0
}
