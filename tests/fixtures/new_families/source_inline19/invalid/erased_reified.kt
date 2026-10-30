inline fun <reified T> test(value: Any): Boolean = value is T
fun <T> bad(value: Any): Boolean = test<T>(value)
