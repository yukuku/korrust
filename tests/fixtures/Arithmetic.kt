package korrust.probes

fun add(a: Int, b: Int): Int = a + b
fun subtract(a: Int, b: Int): Int = a - b
fun multiply(a: Int, b: Int): Int = a * b
fun divide(a: Int, b: Int): Int = a / b
fun remainder(a: Int, b: Int): Int = a % b
fun left(a: Int, b: Int): Int = a shl b
fun right(a: Int, b: Int): Int = a shr b
fun unsignedRight(a: Int, b: Int): Int = a ushr b
fun bitAnd(a: Int, b: Int): Int = a and b
fun bitOr(a: Int, b: Int): Int = a or b
fun bitXor(a: Int, b: Int): Int = a xor b
fun compare(a: Int, b: Int): Int = a.compareTo(b)
fun midpoint(a: Int, b: Int): Int = (a + b) ushr 1
fun precedence(a: Int, b: Int): Int = a + 1 shl b xor 3
fun minimum(): Int = -2147483648
fun maximum(): Int = Int.MAX_VALUE
fun negate(a: Int): Int = -a
fun guarded(flag: Boolean, a: IntArray): Boolean = flag && a[0] > 0
fun orGuarded(flag: Boolean, a: IntArray): Boolean = flag || a[0] > 0
fun invert(flag: Boolean): Boolean = !flag
fun echo(a: IntArray): IntArray = a
fun read(a: IntArray, index: Int): Int = a[index]
fun size(a: IntArray): Int = a.size
fun less(a: Int, b: Int): Boolean = a < b
fun greater(a: Int, b: Int): Boolean = a > b
fun lessEqual(a: Int, b: Int): Boolean = a <= b
fun greaterEqual(a: Int, b: Int): Boolean = a >= b
fun equal(a: Int, b: Int): Boolean = a == b
fun notEqual(a: Int, b: Int): Boolean = a != b
fun conditions(a: Int, b: Int): Boolean = a > b && a != 0 || a == b

fun scope(x: Int): Int {
    var result = x
    if (x > 0) {
        val x = 4
        result = x
    } else {
        val x = 7
        result = x
    }
    return result
}

fun sum(limit: Int): Int {
    var i = 0
    var total = 0
    while (i < limit) {
        total = total + i
        i = i + 1
    }
    return total
}

fun bothReturn(x: Int): Int {
    if (x > 0) {
        return 11
    } else {
        return 13
    }
}

fun recursive(x: Int): Int {
    if (x <= 1) {
        return x
    }
    return recursive(x - 1) + recursive(x - 2)
}
