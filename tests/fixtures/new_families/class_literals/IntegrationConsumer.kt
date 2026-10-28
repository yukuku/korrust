package newfamilies.integrationconsumer

import kotlin.reflect.KClass
import newfamilies.integration.*

fun check(): String {
    val longs: KClass<LongArray> = longArrayType()
    val booleans: KClass<BooleanArray> = booleanArrayType()
    val floats: KClass<FloatArray> = floatArrayType()
    val doubles: KClass<DoubleArray> = doubleArrayType()
    val nestedDoubles: KClass<Array<DoubleArray>> = nestedDoubleArrayType()
    if (longs.java != LongArray::class.java || booleans.java != BooleanArray::class.java) return "integral"
    if (floats.java != FloatArray::class.java || doubles.java != DoubleArray::class.java) return "floating"
    if (nestedDoubles.java != Array<DoubleArray>::class.java) return "nested"
    val scalar: SourceContract<Int> = SourceScalar()
    if (sourceContractAbi(scalar) != java.lang.Integer::class.java) return "generic-abi"
    return "OK"
}
