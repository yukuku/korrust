inline fun <reified T> test(value: Any): Boolean = value is T
fun boundary(value: Any): Boolean = test<List<String>>(value)
