import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.runBlocking
fun limited(block: CoroutineScope.() -> Int): Int = runBlocking(block = block)