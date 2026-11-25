package together

interface Transform {
    fun <T> keep(value: T): T = value
}

class Identity : Transform

abstract class Base<T>(val seed: T) {
    abstract fun read(): T
}

fun source(seed: String): Base<String> = object : Base<String>(seed) {
    override fun read(): String = seed
}

inline fun <T> protect(body: () -> T, after: () -> Unit): T {
    try {
        return body()
    } finally {
        after()
    }
}

fun early(source: Base<String>, identity: Transform, after: () -> Unit): String {
    return protect<String>({ return identity.keep(source.read()) }, after)
}

suspend fun readIf(enabled: Boolean, source: Base<String>, identity: Transform,
                   after: () -> Unit, pause: suspend (String) -> String): String {
    if (enabled) {
        val text = early(source, identity, after)
        return pause(text)
    }
    return "off"
}


suspend fun inlineAfterResume(source: Base<String>, identity: Transform,
                              after: () -> Unit, pause: suspend (String) -> String): String {
    val text = pause(source.read())
    return protect<String>({ return identity.keep(text) }, after)
}
