package receiverfunctions
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.runBlocking
import receiverlibrary.checkpoint
fun compute(): Int = runBlocking {
    val context = coroutineContext
    val base = 5
    val value = checkpoint()
    if (context === coroutineContext) base + value else -1
}
fun captured(seed: Int): Int {
    var total = seed
    runBlocking { val value = checkpoint(); total += value }
    return total
}
fun invokeReceiver(block: suspend CoroutineScope.() -> Int): Int = runBlocking(block = block)

suspend fun applyBlock(scope: CoroutineScope, block: suspend CoroutineScope.() -> Int): Int = scope.block()
fun invokeValue(): Int = runBlocking { applyBlock(this) { checkpoint() } }
fun ordinary(): Int = runBlocking { val action: suspend () -> Int = { checkpoint() }; action() }
