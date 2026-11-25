interface Mapper<T> { fun map(value: T): T }
fun bad(): Mapper<String> = object : Mapper<String> { override fun map(value: Int): String = "bad" }
