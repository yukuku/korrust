package cross

import members.Boundary

class Imported(val next: Boundary) : Boundary {
    override suspend fun read(input: String): String? = next.read(input)
}

suspend fun call(boundary: Boundary, input: String): String? = boundary.retry(input)
