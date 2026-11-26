import api.Source
fun bad(value: Int?): Int = value ?: Source.next()
