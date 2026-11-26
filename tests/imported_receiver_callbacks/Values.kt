package callbackfunctions
import receivercallbacks.adjust as plusReceiver
import receivercallbacks.lease
import receivercallbacks.within
import receivercallbacks.items
import receivercallbacks.numbers
import receivercallbacks.letters
import receivercallbacks.missing
import receiverlibrary.checkpoint
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
fun imported(): List<Int> = within(lease(4)) { (1..3).map { plusReceiver(it) } }
fun asyncs(): List<Int> = runBlocking { (1..3).map { async { checkpoint() } }.awaitAll() }
fun explicit(): List<Int> = runBlocking { (1..3).map { async<Int> { checkpoint() } }.awaitAll() }
fun dispatched(): List<Int> = runBlocking {
    val caller = Thread.currentThread()
    withContext(Dispatchers.Default) {
        (1..3).map { async {
            val result = checkpoint()
            if (Thread.currentThread() === caller) 0 else result
        } }.awaitAll()
    }
}
fun size(): Int = runBlocking { items().size }
fun empty(): Boolean = runBlocking { items().isEmpty() }
fun arraySize(): Int = runBlocking { numbers().size }
fun length(): Int = runBlocking { letters().length }
fun failedLength(): Int = runBlocking { missing()!!.size }
fun shadowed(): List<Int> = within(lease(99)) { (1..3).map {
    val plusReceiver: (Int) -> Int = { value -> value * 2 }
    plusReceiver(it)
} }
