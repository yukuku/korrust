@JvmInline value class Bad(val raw: Int)
fun bad(a: Bad,b: Bad): Boolean = a === b
