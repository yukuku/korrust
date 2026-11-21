import stable.dependency.*
fun bad(value: MutableBox): Int = if (value.text != null) value.text.length else 0
