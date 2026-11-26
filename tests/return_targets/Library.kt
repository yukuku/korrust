package targets
import java.util.concurrent.LinkedBlockingQueue
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine
class Pending(val continuation: Continuation<Int>, val thread: Thread)
private val pending = LinkedBlockingQueue<Pending>()
suspend fun checkpoint(): Int = suspendCoroutine { continuation -> pending.put(Pending(continuation, Thread.currentThread())) }
fun takePending(): Pending = pending.take()
fun resumePending(value: Pending) { value.continuation.resume(9) }
fun plain(block: () -> Int): Int = block()
inline fun cross(crossinline block: () -> Int): Int = block()
inline fun held(noinline block: () -> Int): Int = block()
