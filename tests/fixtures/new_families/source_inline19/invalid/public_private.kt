private fun hidden(x: Int): Int = x
inline fun bad(x: Int): Int = hidden(x)
