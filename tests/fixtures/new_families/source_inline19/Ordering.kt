package ordering
var trace: Int = 0
fun tick(value: Int): Int { trace = trace * 10 + value; return value }
inline fun combine(first: Int, second: Int, block: (Int) -> Int): Int = block(first) + block(second)
fun order(): Int { trace = 0; return combine(second = tick(2), first = tick(1), block = { tick(it + 3) }) }
inline fun divide(value: Int): Int = 100 / value
fun exception(): Int { trace = 0; try { divide(0) } catch (e: ArithmeticException) { return 9 } finally { trace = 7 }; return 0 }
inline fun fail(value: Int): Int { if (value == 0) throw IllegalArgumentException("zero"); return value }
fun throwing(): Int { try { fail(0) } catch (e: IllegalArgumentException) { return 12 }; return 0 }
inline fun publicProperty(): Int = trace
inline fun setPublicProperty(value: Int): Int { trace = value; return trace }
fun property(): Int { setPublicProperty(8); return publicProperty() }
