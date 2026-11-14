class Bad { private operator fun unaryMinus(): Int = 1 }
fun invalid(): Int = -Bad()
