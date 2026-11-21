import stable.dependency.Box
fun bad(value: Box): Int = if (value.text == null) value.text.length else 0
