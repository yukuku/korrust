package receiverlibrary
import java.util.concurrent.LinkedBlockingQueue
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine
class Pending(val continuation: Continuation<Int>, val thread: Thread)
private val pending = LinkedBlockingQueue<Pending>()
suspend fun checkpoint(): Int = suspendCoroutine { continuation -> pending.put(Pending(continuation, Thread.currentThread())) }
fun takePending(): Pending = pending.take()
fun resumePending(value: Pending) { value.continuation.resume(9) }
