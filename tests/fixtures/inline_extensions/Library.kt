package extensions

inline fun String.measure(block: (Int) -> Int): Int = block(length)
inline fun Long.measureLong(block: (Long) -> Long): Long = block(this)
inline fun Double.transform(block: (Double) -> Double): Double = block(this)
inline fun String.simple(): Int = length
inline fun String.forward(extra: Int): Int = simple() + extra
inline fun String?.fallback(block: () -> Int): Int = if (this == null) block() else this!!.length
inline fun Long.retained(noinline block: (Long) -> Long): Long = block(this)
inline fun <reified T> String.accepts(value: Any?): Boolean = value is T && length > 0
