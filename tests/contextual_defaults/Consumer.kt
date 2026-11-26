package fixtures
class Box(val values: List<String?> = emptyList())
fun count(values: List<String?> = emptyList()): Int = values.size
fun first(previous: String, values: List<String> = listOfOne(previous)): String = values[0]
fun checkDefaults(): Boolean = count() == 0 && Box().values.size == 0 && first("ok") == "ok"
