package consumers
fun plain(values: Array<Any>): Int = values.size
fun bad(values: Array<String>): Int = plain(values)
