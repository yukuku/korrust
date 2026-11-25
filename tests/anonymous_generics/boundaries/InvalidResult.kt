interface Mapper<T> { fun map(value: T): T }
fun bad(): Mapper<String> = object : Mapper<String> { override fun map(value: String): Int = 3 }
