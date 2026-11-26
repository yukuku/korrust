package receiverconsumer
import kotlinx.coroutines.runBlocking
import receiverfunctions.invokeReceiver
import receiverlibrary.checkpoint
fun use(): Int = invokeReceiver {
    val context = coroutineContext
    val value = checkpoint()
    if (context === coroutineContext) value + 1 else -1
}
fun nested(): Int = runBlocking {
    val function: suspend kotlinx.coroutines.CoroutineScope.() -> Int = { checkpoint() }
    invokeReceiver(function)
}
