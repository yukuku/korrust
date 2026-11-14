class Input
operator fun Input?.unaryMinus(): Int = 1
fun bounded(value: Input?): Int = -value
