import kotlinx.coroutines.CoroutineScope
inline fun limited(noinline block: suspend CoroutineScope.() -> Int): Int = 1