fun target(required: Int, label: String = "default", block: () -> String): String = label + block()
fun bad(): String = target { "missing required" }
