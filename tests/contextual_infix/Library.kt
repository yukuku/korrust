package dep

class Entry<K, V>(val key: K, val value: V)
infix fun <K,V> K.withValue(value: V): Entry<K,V> = Entry(this, value)
