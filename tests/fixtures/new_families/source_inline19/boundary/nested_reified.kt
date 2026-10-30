inline fun <reified R> test(value: Any): Boolean = value is R
inline fun <reified T> boundary(value: Any): Boolean = test<Array<T>>(value)
