class Input
operator fun Input?.compareTo(other: Input?): Int = 1
fun bounded(value: Input?): Boolean = value < value
