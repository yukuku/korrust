inline fun invoke(crossinline block: () -> Int): Int = block()
fun bad(): Int { invoke { return 1 }; return 2 }
