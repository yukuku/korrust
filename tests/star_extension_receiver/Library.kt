package store
class Box<T>(var item: T)
fun <T> Box<T>.first(): T = item
fun <T> Box<T>.duplicate(): Box<T> = Box(item)
fun <T> Box<T>.replace(value: T) { item = value }
