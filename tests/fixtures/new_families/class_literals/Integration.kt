package newfamilies.integration

import kotlin.reflect.KClass

fun longArrayType(): KClass<LongArray> = LongArray::class
fun booleanArrayType(): KClass<BooleanArray> = kotlin.BooleanArray::class
fun floatArrayType(): KClass<FloatArray> = FloatArray::class
fun doubleArrayType(): KClass<DoubleArray> = kotlin.DoubleArray::class
fun longArrayBound(value: LongArray): Class<out LongArray> = value::class.java
fun nestedDoubleArrayType(): KClass<Array<DoubleArray>> = Array<DoubleArray>::class
interface SourceContract<T> { fun item(): T }
open class SourceScalar : SourceContract<Int> { override fun item(): Int = 700 }
fun scalarAbi(value: SourceScalar): Class<out Int> = value.item()::class.java
fun sourceContractAbi(value: SourceContract<Int>): Class<out Int> = value.item()::class.java

fun subjectless(value: Any?): Int = when { value is String -> value.length; else -> 0 }
fun subjectlessStatement(value: Any?): Int { when { value is String -> return value.length }; return -1 }
fun explicitFalse(value: Any?): Int = when (false) { (value !is String) -> value.length; else -> 0 }
