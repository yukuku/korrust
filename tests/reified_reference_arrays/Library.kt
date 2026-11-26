package erasedallocation

class Box<T>(val value: T)
inline fun <reified T> arbitraryAllocator(size: Int): Array<T?> = arrayOfNulls<T>(size)
inline fun <reified T> arbitraryTest(value: Any): Boolean = value is T
inline fun <reified T> mixedAllocator(size: Int, value: Any): Array<T?> = if (value is T) arrayOfNulls<T>(size) else arrayOfNulls<T>(0)
