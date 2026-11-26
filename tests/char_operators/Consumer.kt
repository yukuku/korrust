package chars

fun difference(left: Char, right: Char): Int = left - right
fun forward(value: Char, offset: Int): Char = value + offset
fun backward(value: Char, offset: Int): Char = value - offset
fun observed(): Char = Steps.nextChar() + Steps.nextOffset()
