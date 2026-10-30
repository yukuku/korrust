package shorts
fun narrow(value: Int): Short = value.toShort()
fun widen(value: Short): Int = value.toInt()
fun longValue(value: Short): Long = value.toLong()
fun floatValue(value: Short): Float = value.toFloat()
fun doubleValue(value: Short): Double = value.toDouble()
fun narrowLong(value: Long): Short = value.toShort()
fun narrowDouble(value: Double): Short = value.toInt().toShort()
fun echo(value: Short?): Short? = value
fun literal(): Short = -32768
fun classifier(): kotlin.reflect.KClass<*> = Short::class
fun bound(value: Short): kotlin.reflect.KClass<*> = value::class
