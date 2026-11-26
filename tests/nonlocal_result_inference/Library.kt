package scope
inline fun <T> fallback(block: () -> T): T = block()
inline fun <T> restricted(crossinline block: () -> T): T = block()
