package maker
@Suppress("UNCHECKED_CAST")
fun <T : Any> select(value: Any): T = value as T
@Suppress("UNCHECKED_CAST")
fun <T : CharSequence> chars(value: Any): T = value as T
