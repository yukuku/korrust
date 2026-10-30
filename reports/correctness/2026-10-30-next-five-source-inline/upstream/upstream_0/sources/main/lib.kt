// boxed primitive comparisons
fun isBoolean(a: Any) = a::class == true::class
fun isChar(a: Any) = a::class == 'c'::class
fun isByte(a: Any) = Byte::class == a::class
fun isShort(a: Any) = 1.toShort()::class == a::class
fun isInt(a: Any) = a::class == (40 + 2)::class
fun isLong(a: Any) = a::class == 0L::class
fun isFloat(a: Any) = a::class == 1.4f::class
fun isDouble(a: Any) = a::class == 0.0::class

// reified primitive comparisons
inline fun <reified T> isReifiedInt() = 1::class == T::class

