package arrays
inline fun <reified T: Any> values(): Array<T> = emptyArray<T>()
fun size(vararg values: String): Int = values.size
fun objects(vararg values: Any?): Int = values.size
