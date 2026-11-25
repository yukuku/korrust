interface Base<T>
fun <T> make(): Base<List<T>> = object : Base<List<T>> {}
