inline fun <reified R> test(value: Any): Boolean = value is R
fun <T> bad(value: Any): Boolean = test<Array<T>>(value)
