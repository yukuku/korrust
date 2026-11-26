package caller
import scope.fallback
class Counter(var hits: Int)
fun length(value: String?, counter: Counter): Int {
    val text = value ?: fallback {
        counter.hits = counter.hits + 1
        return -1
    }
    return text.length
}
