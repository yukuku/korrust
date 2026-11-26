import library.*
fun bad(): String = inScope { combine(first(listOf(1)), 7) }
