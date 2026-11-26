import kotlinx.coroutines.CoroutineScope
suspend fun bad(scope: CoroutineScope, block: (suspend CoroutineScope.() -> Int)?): Int = block(scope)