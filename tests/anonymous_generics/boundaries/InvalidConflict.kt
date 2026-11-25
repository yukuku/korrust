interface Mapper<T> { fun map(value: T): T }
open class Base : Mapper<String> { override fun map(value: String): String = value }
fun bad(): Mapper<Int> = object : Base(), Mapper<Int> { override fun map(value: Int): Int = value }
