package next.probes

fun whenType(value: Any?): Int = when (value) { is String -> value.length; !is CharSequence -> -2; else -> -3 }
fun whenBooleanType(value: Any?): Int = when (true) { (value is String) -> 1; else -> 0 }
fun subjectlessType(value: Any?): Int = when { value is String -> value.length; else -> -1 }
