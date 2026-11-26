@Suppress("UNCHECKED_CAST")
fun checked(value: Any?): List<String> = value as List<String>
@Suppress("UNCHECKED_CAST")
fun first(value: Any?): String = (value as List<String>)[0]
@Suppress("UNCHECKED_CAST")
fun safe(value: Any?): List<String>? = value as? List<String>
