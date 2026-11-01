package consumer

import suspension.*
import kotlin.coroutines.*

fun main() {
    var first: Continuation<String>? = null
    var second: Continuation<Long>? = null
    var outcome: String? = null
    var completed = 0
    val operation: suspend () -> String = {
        combine(3, { input ->
            check(input == 3)
            suspendCoroutine { continuation -> first = continuation }
        }, { input ->
            check(input == "K-6")
            suspendCoroutine { continuation -> second = continuation }
        })
    }
    operation.startCoroutine(object : Continuation<String> {
        override val context: CoroutineContext = EmptyCoroutineContext
        override fun resumeWith(result: Result<String>) {
            outcome = result.getOrThrow()
            completed++
        }
    })
    check(completed == 0)
    first!!.resume("K")
    check(completed == 0)
    second!!.resume(11L)
    check(completed == 1)
    check(outcome == "K-6:11")
    println("OK")
}
