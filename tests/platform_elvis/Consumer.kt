package use
import api.Source
fun maybe(value: String?): String? = value ?: Source.next()
fun required(value: String?): String = value ?: Source.next()
fun field(value: String?): String? = value ?: Source.fallback
