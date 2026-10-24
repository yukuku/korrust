package ten

fun longAdd(a: Long, b: Long): Long = a + b
fun longSubtract(a: Long, b: Long): Long = a - b
fun longMultiply(a: Long, b: Long): Long = a * b
fun longDivide(a: Long, b: Long): Long = a / b
fun longRemainder(a: Long, b: Long): Long = a % b
fun longAnd(a: Long, b: Long): Long = a and b
fun longOr(a: Long, b: Long): Long = a or b
fun longXor(a: Long, b: Long): Long = a xor b
fun longLeft(a: Long, b: Int): Long = a shl b
fun longRight(a: Long, b: Int): Long = a shr b
fun longUnsignedRight(a: Long, b: Int): Long = a ushr b
fun longNegate(a: Long): Long = -a
fun longLess(a: Long, b: Long): Boolean = a < b
fun longLessEqual(a: Long, b: Long): Boolean = a <= b
fun longGreater(a: Long, b: Long): Boolean = a > b
fun longGreaterEqual(a: Long, b: Long): Boolean = a >= b
fun longEqual(a: Long, b: Long): Boolean = a == b
fun longNotEqual(a: Long, b: Long): Boolean = a != b
fun longText(a: Long): String = a.toString()
fun longMinimum(): Long = -9223372036854775807L - 1L
fun longMaximum(): Long = 9223372036854775807L
fun longConstants(): Long = Long.MIN_VALUE xor Long.MAX_VALUE
fun longHex(): Long = 0x7fff_ffff_ffff_ffffL
fun longBinary(): Long = 0b1010_0101L
fun longExpectedLiteral(): Long = 17
fun longSlots(a: Long, flag: Boolean, b: Long, delta: Int): Long {
    var result: Long = a
    val middle: Long = b + 3L
    if (flag) { result = result + middle } else { result = result - middle }
    if (delta < 0) { result = -result }
    return result
}
