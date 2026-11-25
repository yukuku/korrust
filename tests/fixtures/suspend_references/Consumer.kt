package consumer

import kotlin.coroutines.*
import suspendrefs.*

fun main() {
    val pending = ArrayList<Continuation<String>>()
    val worker = Worker("K") { value -> suspendCoroutine<String> { pending.add(it) } }
    val reference = boundReference(worker)
    var result: String? = null
    val block: suspend () -> String = { reference("x") }
    block.startCoroutine(object : Continuation<String> {
        override val context: CoroutineContext = EmptyCoroutineContext
        override fun resumeWith(value: Result<String>) { result = value.getOrThrow() }
    })
    check(result == null && pending.size == 1)
    pending.removeAt(0).resume("first")
    check(result == null && pending.size == 1)
    pending.removeAt(0).resume("last")
    check(result == "Klast")
    println("OK")
}
