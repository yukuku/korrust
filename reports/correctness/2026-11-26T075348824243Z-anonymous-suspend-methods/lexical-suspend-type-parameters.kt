import anonymouslibrary.Lexical
fun <T> use(value: T): Lexical<T> = object : Lexical<T> { override suspend fun value(): T = value }