inline fun invoke(noinline block: () -> Int): Int = block()
fun bad(): Int { invoke { return 1 }; return 2 }
