import stable.dependency.*
fun bad(first: Box, second: Box): Int = if (first.text != null) second.text.length else 0
