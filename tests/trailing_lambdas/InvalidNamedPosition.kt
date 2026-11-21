fun target(prefix: String = "default", block: () -> String): String = prefix + block()
fun bad(): String = target(block = { "named" }, "out of order")
