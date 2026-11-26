package branches

fun nested(n: Int): String {
    val selected = try {
        val zero = Steps.checked(n)
        if (zero) "zero" else if (n == 1) { "one" } else "many"
    } catch (e: IllegalStateException) {
        if (n == -1) "minus one" else "negative"
    } finally {
        Steps.cleanups = Steps.cleanups + 1
    }
    return selected
}
fun numbers(n: Int): Int = try {
    if (Steps.checked(n)) 7 else 9
} catch (e: IllegalStateException) { 11 }
