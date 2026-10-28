package arrayconsumer
import newfamilies.arrays.*
fun main() {
val valuesLong: LongArray = initLong(3) { index -> index.toLong() }
check(valuesLong.size == 3)
check(keepLong(valuesLong) === valuesLong)
check(spreadLong(valuesLong).size == 5)
check(nullableLong(null).isEmpty())
val valuesBoolean: BooleanArray = initBoolean(3) { index -> index % 2 == 0 }
check(valuesBoolean.size == 3)
check(keepBoolean(valuesBoolean) === valuesBoolean)
check(spreadBoolean(valuesBoolean).size == 5)
check(nullableBoolean(null).isEmpty())
val valuesFloat: FloatArray = initFloat(3) { index -> index.toFloat() }
check(valuesFloat.size == 3)
check(keepFloat(valuesFloat) === valuesFloat)
check(spreadFloat(valuesFloat).size == 5)
check(nullableFloat(null).isEmpty())
val valuesDouble: DoubleArray = initDouble(3) { index -> index.toDouble() }
check(valuesDouble.size == 3)
check(keepDouble(valuesDouble) === valuesDouble)
check(spreadDouble(valuesDouble).size == 5)
check(nullableDouble(null).isEmpty())
val holder = ArrayHolder(valuesLong, valuesBoolean, valuesFloat, valuesDouble)
check(holder.longs === valuesLong && holder.booleans === valuesBoolean && holder.floats === valuesFloat && holder.doubles === valuesDouble)
check(typeNonnull(valuesDouble) === valuesDouble && typeNullable(null) == null)
check(typeText("text") == "text" && typeNullableText(null) == null)
println("OK")
}
