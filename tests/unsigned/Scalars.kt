package unsignedcases

fun uintZero(): UInt = 0u
fun uintMax(): UInt = 0xffff_ffffU
fun ulongMax(): ULong = 18_446_744_073_709_551_615uL
fun inferredWide() = 4294967296u
fun contextualWide(): ULong = 1u
fun binaryLiteral(): UInt = 0b1111_0000u
fun uintAdd(a: UInt, b: UInt): UInt = a + b
fun uintSubtract(a: UInt, b: UInt): UInt = a - b
fun uintMultiply(a: UInt, b: UInt): UInt = a * b
fun uintDivide(a: UInt, b: UInt): UInt = a / b
fun uintRemainder(a: UInt, b: UInt): UInt = a % b
fun ulongAdd(a: ULong, b: ULong): ULong = a + b
fun ulongSubtract(a: ULong, b: ULong): ULong = a - b
fun ulongMultiply(a: ULong, b: ULong): ULong = a * b
fun ulongDivide(a: ULong, b: ULong): ULong = a / b
fun ulongRemainder(a: ULong, b: ULong): ULong = a % b
fun mixedAdd(a: UInt, b: ULong): ULong = a + b
fun addLiteral(a: UInt): UInt = a + 1u
fun uintCompare(a: UInt, b: UInt): Int = a.compareTo(b)
fun ulongCompare(a: ULong, b: ULong): Int = a.compareTo(b)
fun uintLess(a: UInt, b: UInt): Boolean = a < b
fun ulongAtLeast(a: ULong, b: ULong): Boolean = a >= b
fun uintAnd(a: UInt, b: UInt): UInt = a and b
fun uintOr(a: UInt, b: UInt): UInt = a or b
fun uintXor(a: UInt, b: UInt): UInt = a xor b
fun uintShift(a: UInt, count: Int): UInt = a shr count
fun uintShiftLeft(a: UInt, count: Int): UInt = a shl count
fun ulongXor(a: ULong, b: ULong): ULong = a xor b
fun ulongShift(a: ULong, count: Int): ULong = a shr count
fun uintInvert(a: UInt): UInt = a.inv()
fun ulongInvert(a: ULong): ULong = a.inv()
fun intToUInt(a: Int): UInt = a.toUInt()
fun longToULong(a: Long): ULong = a.toULong()
fun uintToLong(a: UInt): Long = a.toLong()
fun ulongToInt(a: ULong): Int = a.toInt()
fun uintToULong(a: UInt): ULong = a.toULong()
fun ulongToUInt(a: ULong): UInt = a.toUInt()
fun smallByte(a: UInt): UByte = a.toUByte()
fun smallShort(a: ULong): UShort = a.toUShort()
fun byteToUInt(a: UByte): UInt = a.toUInt()
fun shortToULong(a: UShort): ULong = a.toULong()
fun uintText(a: UInt): String = a.toString()
fun ulongText(a: ULong): String = a.toString()
fun uintNullable(a: UInt?): UInt? = a
fun uintElvis(a: UInt?): UInt = a ?: 7u
fun nullableEquals(a: UInt?): Boolean = a == 0u
fun literalEqualsNullable(a: ULong?): Boolean = 0uL == a
fun uintBox(a: UInt): Any = a
fun ulongBox(a: ULong): Any = a
fun uintList(a: UInt): List<UInt> = listOf(a)
fun <T> identity(value: T): T = value
fun uintGeneric(a: UInt): UInt = identity(a)
fun ulongGeneric(a: ULong): ULong = identity(a)
fun failUInt(): UInt = throw IllegalStateException("rhs")
fun ordered(log: StringBuilder): UInt = mark(log, "L", 0xffff_ffffu) + mark(log, "R", 1u)
fun mark(log: StringBuilder, label: String, value: UInt): UInt {
    log.append(label)
    return value
}
fun orderedFailure(log: StringBuilder): UInt = mark(log, "L", 1u) / failUInt()
class ScalarBox(val first: UInt, val second: ULong)
data class ScalarPair(val first: UInt, val second: ULong)
fun lambdaCapture(a: UInt): () -> UInt = { a + 1u }
fun uintIdentityNull(a: UInt?): Boolean = a === null
fun ulongIdentityPresent(a: ULong?): Boolean = null !== a
fun separatedSuffix(): UInt = 1_u
fun inferredAdd(a: UInt, b: UInt) = a + b
fun inferredConversion(a: ULong) = a.toInt()
operator fun UInt.plus(other: Int): UInt = this + other.toUInt()
fun extensionCall(a: UInt): UInt = a + 1
