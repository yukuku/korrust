inline fun <reified R> test(value: Any): Boolean = value is R
fun boundary(value: Any): Boolean = test<Array<List<String>>>(value)
