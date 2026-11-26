package consumers

import dep.withValue
import dep.Entry

fun inferred(): Entry<String, List<String>> = "a" withValue emptyList()
fun values(): Map<String, List<String>?> =
    linkedMapOf("a" to listOf("one"), "n" to null, "e" to emptyList(), "b" to listOf("two"))
