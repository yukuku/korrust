import kotlinx.coroutines.CoroutineScope
fun bad(scope: CoroutineScope, block: suspend CoroutineScope.() -> Int): Int = block(scope)