package dep

class Entry<K, V>(val key: K, val value: V)
fun <K, V> lastValue(vararg entries: Entry<K, V>): V = entries.last().value
