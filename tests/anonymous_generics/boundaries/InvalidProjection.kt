interface Mapper<T> { fun map(value: T): T }
fun bad(): Mapper<out String> = object : Mapper<out String> { override fun map(value: String): String = value }
