import stable.dependency.*
fun bad(first: Box, second: Box): Int { var value = first; if (value.text != null) { value = second; return value.text.length }; return 0 }
