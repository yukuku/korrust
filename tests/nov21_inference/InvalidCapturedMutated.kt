package invalid
import inference.dependency.exists
fun bad(input: String?): Int {
    var value = input
    val mutate = { value = null }
    if (exists(value)) {
        mutate()
        return value.length
    }
    return 0
}
