import stable.dependency.*
fun bad(value: OpenBox): Int = if (value.text != null) value.text.length else 0
