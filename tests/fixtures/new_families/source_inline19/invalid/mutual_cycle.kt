inline fun one(x: Int): Int = two(x)
inline fun two(x: Int): Int = one(x)
