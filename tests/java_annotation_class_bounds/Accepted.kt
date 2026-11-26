package annotationboundsuses
import annotationbounds.Bounds
import annotationbounds.Upper
import annotationbounds.Nested
import kotlin.reflect.KClass

@Bounds class Defaults
@Bounds(error = IllegalArgumentException::class, exact = Number::class, lower = Any::class, any = Int::class, arrays = IntArray::class, serial = Array<String>::class, integer = Int::class)
class Explicit
@Upper(value = [Int::class, Long::class, Number::class]) class Numbers
@Upper class DefaultNumbers
@Upper(value = []) class Empty
@Upper(Int::class) class Single
@Upper(*[Long::class]) class Spread
@Nested(Upper(value = [Double::class])) class Recursive
fun errorClass(value: Bounds): KClass<out Throwable> = value.error
fun exactClass(value: Bounds): KClass<Number> = value.exact
fun lowerClass(value: Bounds): KClass<in Number> = value.lower
fun integerClass(value: Bounds): KClass<Int> = value.integer
fun arrayCount(value: Upper): Int = value.value.size
