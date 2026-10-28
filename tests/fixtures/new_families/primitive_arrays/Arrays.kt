package newfamilies.arrays
import java.util.Arrays
import newfamilies.arrays.ArrayEffects

fun defaultsLong(n: Int): LongArray = LongArray(n)
fun initLong(n: Int, initializer: (Int) -> Long): LongArray = LongArray(n, initializer)
fun getLong(value: LongArray, index: Int): Long = value[index]
fun putLong(value: LongArray, index: Int, item: Long): Long { val old = value[index]; value[index] = item; return old }
fun keepLong(value: LongArray): LongArray = value
fun nullableLong(value: LongArray?): LongArray = value ?: LongArray(0)
fun packLong(vararg values: Long): LongArray = values
fun spreadLong(value: LongArray): LongArray = packLong(1L, *value, 3L)
fun builtinLong(): LongArray = longArrayOf(1L, 2L, 3L)
fun copyLong(value: LongArray, n: Int): LongArray = Arrays.copyOf(value, n)
fun orderedLong() { ArrayEffects.longArray()[ArrayEffects.index()] = ArrayEffects.longValue() }
fun sumLong(value: LongArray): Long { var sum = 0L; for (item in value) { sum += item }; return sum }
fun defaultsBoolean(n: Int): BooleanArray = BooleanArray(n)
fun initBoolean(n: Int, initializer: (Int) -> Boolean): BooleanArray = BooleanArray(n, initializer)
fun getBoolean(value: BooleanArray, index: Int): Boolean = value[index]
fun putBoolean(value: BooleanArray, index: Int, item: Boolean): Boolean { val old = value[index]; value[index] = item; return old }
fun keepBoolean(value: BooleanArray): BooleanArray = value
fun nullableBoolean(value: BooleanArray?): BooleanArray = value ?: BooleanArray(0)
fun packBoolean(vararg values: Boolean): BooleanArray = values
fun spreadBoolean(value: BooleanArray): BooleanArray = packBoolean(true, *value, true)
fun builtinBoolean(): BooleanArray = booleanArrayOf(true, false, true)
fun copyBoolean(value: BooleanArray, n: Int): BooleanArray = Arrays.copyOf(value, n)
fun orderedBoolean() { ArrayEffects.booleanArray()[ArrayEffects.index()] = ArrayEffects.booleanValue() }
fun countBoolean(value: BooleanArray): Int { var count = 0; for (item in value) { if (item) count += 1 }; return count }
fun defaultsFloat(n: Int): FloatArray = FloatArray(n)
fun initFloat(n: Int, initializer: (Int) -> Float): FloatArray = FloatArray(n, initializer)
fun getFloat(value: FloatArray, index: Int): Float = value[index]
fun putFloat(value: FloatArray, index: Int, item: Float): Float { val old = value[index]; value[index] = item; return old }
fun keepFloat(value: FloatArray): FloatArray = value
fun nullableFloat(value: FloatArray?): FloatArray = value ?: FloatArray(0)
fun packFloat(vararg values: Float): FloatArray = values
fun spreadFloat(value: FloatArray): FloatArray = packFloat(1.25f, *value, 3.75f)
fun builtinFloat(): FloatArray = floatArrayOf(1.25f, 2.5f, 3.75f)
fun copyFloat(value: FloatArray, n: Int): FloatArray = Arrays.copyOf(value, n)
fun orderedFloat() { ArrayEffects.floatArray()[ArrayEffects.index()] = ArrayEffects.floatValue() }
fun sumFloat(value: FloatArray): Float { var sum = 0.0f; for (item in value) { sum += item }; return sum }
fun defaultsDouble(n: Int): DoubleArray = DoubleArray(n)
fun initDouble(n: Int, initializer: (Int) -> Double): DoubleArray = DoubleArray(n, initializer)
fun getDouble(value: DoubleArray, index: Int): Double = value[index]
fun putDouble(value: DoubleArray, index: Int, item: Double): Double { val old = value[index]; value[index] = item; return old }
fun keepDouble(value: DoubleArray): DoubleArray = value
fun nullableDouble(value: DoubleArray?): DoubleArray = value ?: DoubleArray(0)
fun packDouble(vararg values: Double): DoubleArray = values
fun spreadDouble(value: DoubleArray): DoubleArray = packDouble(1.25, *value, 3.75)
fun builtinDouble(): DoubleArray = doubleArrayOf(1.25, 2.5, 3.75)
fun copyDouble(value: DoubleArray, n: Int): DoubleArray = Arrays.copyOf(value, n)
fun orderedDouble() { ArrayEffects.doubleArray()[ArrayEffects.index()] = ArrayEffects.doubleValue() }
fun sumDouble(value: DoubleArray): Double { var sum = 0.0; for (item in value) { sum += item }; return sum }

fun floatTemplate(value: Float): String = "[$value]"
fun doubleTemplate(value: Double?): String = "[$value]"
fun byteTemplate(value: Byte): String = "[$value]"

class ArrayHolder(val longs: LongArray, val booleans: BooleanArray, val floats: FloatArray, val doubles: DoubleArray)

fun spreadOnlyLong(value: LongArray): LongArray = packLong(*value)

fun spreadOnlyBoolean(value: BooleanArray): BooleanArray = packBoolean(*value)

fun spreadOnlyFloat(value: FloatArray): FloatArray = packFloat(*value)

fun spreadOnlyDouble(value: DoubleArray): DoubleArray = packDouble(*value)

fun framedLong(flag: Boolean): LongArray { val values = LongArray(2); if (flag) values[1] = 7L; return values }

fun framedBoolean(flag: Boolean): BooleanArray { val values = BooleanArray(2); if (flag) values[1] = true; return values }

fun framedFloat(flag: Boolean): FloatArray { val values = FloatArray(2); if (flag) values[1] = 7.5f; return values }

fun framedDouble(flag: Boolean): DoubleArray { val values = DoubleArray(2); if (flag) values[1] = 7.5; return values }
