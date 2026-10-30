fun existing(value: Any?): Int = when (false) { value !is String -> value.length; else -> 0 }
