package consumers
import dep.Entry
import dep.lastValue
fun bad(a: Entry<String, Int>, b: Entry<String, Long>): String = lastValue(a, b)
