package consumer

import kotlin.coroutines.*
import members.*

fun main() {
    val pending = ArrayList<Continuation<String>>()
    var value: String? = null
    var completed = false
    val reader = Reader("K") { input ->
        suspendCoroutine<String> { continuation -> pending.add(continuation) }
    }
    val block: suspend () -> String? = { dispatch(reader, "x") }
    block.startCoroutine(object : Continuation<String?> {
        override val context: CoroutineContext = EmptyCoroutineContext
        override fun resumeWith(result: Result<String?>) {
            value = result.getOrThrow()
            completed = true
        }
    })
    check(!completed && pending.size == 1)
    pending.removeAt(0).resume("first")
    check(!completed && pending.size == 1)
    pending.removeAt(0).resume("last")
    check(completed && value == "Klast1" && reader.visits == 1)
    val boundary = Boundary { input -> input + "!" }
    var immediate: String? = null
    val retry: suspend () -> String? = { boundary.retry("sam") }
    retry.startCoroutine(object : Continuation<String?> {
        override val context: CoroutineContext = EmptyCoroutineContext
        override fun resumeWith(result: Result<String?>) { immediate = result.getOrThrow() }
    })
    check(immediate == "sam!")
    println("OK")
}
