package members

fun decorate(value: String): String = value + ":"

suspend fun sourceFetch(value: String, callback: suspend (String) -> String): String = callback(value)

fun interface Boundary {
    suspend fun read(input: String): String?
    suspend fun retry(input: String): String? = read(input)
}

class Reader(val prefix: String, val callback: suspend (String) -> String) : Boundary {
    var visits: Int = 0

    override suspend fun read(input: String): String {
        visits = visits + 1
        val start = prefix + decorate(input)
        val result = callback(start)
        val second = sourceFetch(result, callback)
        return prefix + second + visits.toString()
    }
}

class Immediate(val prefix: String) : Boundary {
    override suspend fun read(input: String): String? = prefix + input
}

class Scalars(val bias: Long) {
    suspend fun combine(value: Long, fraction: Double, callback: suspend (Long) -> Long): Long {
        val result = callback(value)
        return bias + result + fraction.toLong()
    }
}

suspend fun dispatch(boundary: Boundary, input: String): String? = boundary.retry(input)

suspend fun concrete(reader: Reader, input: String): String = reader.read(input)

suspend fun dependency(boundary: suspend_dependency.External, input: String): String? = boundary.read(input)

class ExternalReader(val callback: suspend (String) -> String) : suspend_dependency.External {
    override suspend fun read(input: String): String? = callback(input)
}

abstract class AbstractBoundary {
    abstract suspend fun read(input: String): String?
}
