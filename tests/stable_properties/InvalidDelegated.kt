import stable.dependency.*
fun bad(value: DelegatedBox): Int = if (value.text != null) value.text.length else 0
