import stable.dependency.*
fun bad(first: Box, second: Box): Int { if (first.text != null) { val first = second; return first.text.length }; return 0 }
