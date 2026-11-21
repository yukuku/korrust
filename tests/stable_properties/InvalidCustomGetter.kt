import stable.dependency.*
fun bad(value: CustomBox): Int = if (value.text != null) value.text.length else 0
