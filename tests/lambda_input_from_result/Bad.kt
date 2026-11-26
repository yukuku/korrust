import checks.read
fun bad(): String? = read("x") { it.missingMethod() }
