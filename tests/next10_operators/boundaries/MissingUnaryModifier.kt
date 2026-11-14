class Bad { fun unaryMinus(): Int = 1 }
fun invalid(): Int = -Bad()
