import stable.dependency.*
fun bad(value: Box): Int { if (value.text != null) { value.text.length }; return value.text.length }
