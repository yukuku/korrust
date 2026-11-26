package consumers

import dep.Entry
import dep.lastValue

fun inferred(a: Entry<String, Long>, b: Entry<String, Long>): Long = lastValue(a, b)
fun spread(a: Entry<Int, String>, rest: Array<Entry<Int, String>>): String = lastValue(a, *rest)
fun words(): Map<String, String> = mapOf("a" to "one", "b" to "two")
