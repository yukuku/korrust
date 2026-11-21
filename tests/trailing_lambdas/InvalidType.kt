import trailing.dependency.apply
fun bad(): String? = apply(1) { value: String -> value }
