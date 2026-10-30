package forward
import probe.test as importedTest
inline fun <reified U> relay(value: Any?): Boolean = importedTest<U>(value)
inline fun outer(x: Int): Int = inner(x) + inner(x + 1)
inline fun inner(x: Int): Int = x * 3
fun answer(): Int = outer(2)
fun yes(value: Any?): Boolean = relay<String>(value)
