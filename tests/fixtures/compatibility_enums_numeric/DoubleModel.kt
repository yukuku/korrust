package compatibility.enumsnumeric

class DoubleBox(var value: Double, val optional: Double?) {
    fun add(delta: Double): Double {
        value += delta
        return value
    }
}

data class DoubleRecord(val value: Double, val optional: Double?)

object DoubleModel {
    @JvmStatic fun scalar(value: Double): Double = value
    @JvmStatic fun boxed(value: Double?): Double? = value
    @JvmStatic fun primitiveEqual(first: Double, second: Double): Boolean = first == second
    @JvmStatic fun boxedEqual(first: Double?, second: Double?): Boolean = first == second
    @JvmStatic fun genericEqual(first: Any?, second: Any?): Boolean = first == second
    @JvmStatic fun less(first: Double, second: Double): Boolean = first < second
    @JvmStatic fun lessEqual(first: Double, second: Double): Boolean = first <= second
    @JvmStatic fun greater(first: Double, second: Double): Boolean = first > second
    @JvmStatic fun greaterEqual(first: Double, second: Double): Boolean = first >= second
    @JvmStatic fun toInt(value: Double): Int = value.toInt()
    @JvmStatic fun toLong(value: Double): Long = value.toLong()
    @JvmStatic fun toFloat(value: Double): Float = value.toFloat()
    @JvmStatic fun fromInt(value: Int): Double = value.toDouble()
    @JvmStatic fun fromLong(value: Long): Double = value.toDouble()
    @JvmStatic fun fromFloat(value: Float): Double = value.toDouble()
    @JvmStatic fun arithmetic(first: Double, second: Double): Double = ((first + second) * second - first) / second % 3.0
    @JvmStatic fun mixed(first: Double, second: Int, third: Long, fourth: Float): Double = first + second + third + fourth
    @JvmStatic fun slots(first: Int, second: Double, third: Long, fourth: Double, choose: Boolean): Double {
        val selected: Double = if (choose) second else fourth
        val other: Double = if (choose) fourth else second
        return selected + other + first.toDouble() + third.toDouble()
    }
    @JvmStatic fun nullable(value: Double?, fallback: Double): Double = value ?: fallback
    @JvmStatic fun asserted(value: Double?): Double = value!!
    @JvmStatic fun minLiteral(): Double = 4.9e-324
    @JvmStatic fun maxLiteral(): Double = 1.7976931348623157e308
    @JvmStatic fun overflowLiteral(): Double = 1.0e400
    @JvmStatic fun underflowLiteral(): Double = 1.0e-400
    @JvmStatic fun genericRoundTrip(value: Double): Double = identity(value)
    @JvmStatic fun zeroLiteral(): Double = -0.0
    @JvmStatic fun listAssert(value: List<Double?>): Double = value[0]!!
    @JvmStatic fun listNullable(value: List<Double?>): Double? = value[0]
    @JvmStatic fun listLocalAssert(value: List<Double?>): Double { val current = value[0]; return current!! }
    @JvmStatic fun listElvis(value: List<Double?>): Double = value[0] ?: 2.0
    @JvmStatic fun listValue(value: List<Double>): Double = value[0]
    @JvmStatic fun javaCalls(first: Double, second: Double): Double = java.lang.Math.max(java.lang.Math.abs(first), java.lang.Math.sqrt(second))
}

fun <T> identity(value: T): T = value
