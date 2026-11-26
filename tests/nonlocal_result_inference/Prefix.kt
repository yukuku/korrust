package caller
import scope.fallback
fun bad(value: String?): Int {
    val text = value ?: fallback { missing(); return -1 }
    return text.length
}
