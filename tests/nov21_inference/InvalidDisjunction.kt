package invalid
import inference.dependency.either
fun bad(a: String?, b: String?): Int {
    if (either(a, b)) return a.length
    return 0
}
