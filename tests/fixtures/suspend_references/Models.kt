package suspendrefs

suspend fun forward(value: String, step: suspend (String) -> String): String = step(value)

fun topReference(): suspend (String, suspend (String) -> String) -> String = ::forward

interface Boundary {
    suspend fun read(value: String): String
}

class Worker(val prefix: String, val step: suspend (String) -> String) : Boundary {
    override suspend fun read(value: String): String {
        val first = step(prefix + value)
        val second = step(first)
        return prefix + second
    }
}

class Holder(val worker: Worker) {
    var visits: Int = 0
    fun next(): Worker {
        visits = visits + 1
        return worker
    }
}

fun boundReference(worker: Worker): suspend (String) -> String = worker::read
fun unboundReference(): suspend (Worker, String) -> String = Worker::read
fun interfaceReference(worker: Boundary): suspend (String) -> String = worker::read
fun evaluatedReference(holder: Holder): suspend (String) -> String = holder.next()::read
fun nullableReference(worker: Worker): (suspend (String) -> String)? = worker::read
fun ordinary(value: Int): Int = value + 7
fun ordinaryReference(): suspend (Int) -> Int = ::ordinary
suspend fun textLength(value: String): Int = value.length
fun scalarReference(): suspend (String) -> Int = ::textLength
suspend fun consume(value: String): Unit { }
fun unitReference(): suspend (String) -> Unit = ::consume
private suspend fun hidden(value: String): String = value + "!"
fun privateReference(): suspend (String) -> String = ::hidden
