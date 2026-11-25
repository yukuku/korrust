interface Mapper<T : CharSequence> { fun map(value: T): T }
fun bad(): Mapper<Int> = object : Mapper<Int> { override fun map(value: Int): Int = value }
