import checks.read
fun text(): String? = read("abcd") { it.length == 4 }
fun count(): Int = read(7) { it == 7 }
