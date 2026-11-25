package composition

abstract class Source(val prefix: String) {
    abstract fun read(): String
}

class Cache<C>(val first: C) {
    fun <T> save(value: T): () -> T = { value }
}

fun reader(value: String): Source {
    val saved = Cache<Int>(7).save(value)
    return object : Source("verse ") {
        override fun read(): String = prefix + saved()
    }
}

suspend fun fetch(source: Source, step: suspend (String) -> String): String {
    suspend fun next(): String = step(source.read())
    return next()
}

fun reference(): suspend (Source, suspend (String) -> String) -> String = ::fetch
