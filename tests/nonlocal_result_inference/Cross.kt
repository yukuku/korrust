package caller
import scope.restricted
fun bad(value: String?): Int {
    val text = value ?: restricted { return -1 }
    return text.length
}
