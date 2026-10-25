package next.probes

fun stringTest(value: Any?): Boolean = value is String
fun notStringTest(value: Any?): Boolean = value !is String
fun sequenceTest(value: Any?): Boolean = value is CharSequence
fun nullableStringTest(value: Any?): Boolean = value is String?
