inline fun <R> invoke(block: () -> R): R = block()
fun boundary(): Int { invoke { return 1 }; return 2 }
