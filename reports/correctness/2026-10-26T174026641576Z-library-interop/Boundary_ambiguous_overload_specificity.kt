fun ambiguous(a: String, b: Any, c: Any): Int = 1
fun ambiguous(a: Any, b: String, c: String): Int = 2
fun bad(): Int = ambiguous("a", "b", "c")
