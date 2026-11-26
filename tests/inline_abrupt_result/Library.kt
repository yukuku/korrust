package scoped
inline fun <T, R> T.runValue(block: (T) -> R): R = block(this)
fun <T, R> T.deferred(block: (T) -> R): R = block(this)
inline fun <T, R> T.cross(crossinline block: (T) -> R): R = block(this)
