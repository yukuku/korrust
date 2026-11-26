package caller
import scope.fallback
fun bad(value: String?): Int {
    val text = value ?: fallback { return "wrong" }
    return text.length
}
