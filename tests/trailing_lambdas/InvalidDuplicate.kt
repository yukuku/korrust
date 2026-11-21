fun target(block: () -> String): String = block()
fun bad(): String = target(block = { "first" }) { "second" }
