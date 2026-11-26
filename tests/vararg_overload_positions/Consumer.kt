package consumers

import java.util.Locale
import overloads.pick
import overloads.select

fun empty(): String = pick()
fun any(first: Any): String = pick(first, 3)
fun narrow(): String = pick("p", 3, null)
fun spread(items: Array<Any?>): String = pick("s", *items)
fun named(items: Array<Any?>): String = pick(items = items, prefix = "n")
fun numbers(): String = select(2, 3)
fun formatted(): String = (0 until 3).joinToString(",", "[", "]") { "%06x".format(Locale.ROOT, it) }
