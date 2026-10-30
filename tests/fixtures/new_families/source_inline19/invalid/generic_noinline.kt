inline fun <R> invoke(noinline block: () -> R): R = block()
fun bad(value: Boolean): Int { invoke { if (value) return 1; 2 }; return 3 }
