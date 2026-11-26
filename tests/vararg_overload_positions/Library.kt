package overloads

fun pick(vararg items: Any?): String = "all:" + items.size
fun pick(prefix: String, vararg items: Any?): String = prefix + ":" + items.size
fun select(vararg items: Number): String = "numbers:" + items.size
fun select(first: Int, vararg items: Number): String = "int:" + first + ":" + items.size
