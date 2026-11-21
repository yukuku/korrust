import stable.dependency.*
fun next(): Box = Box(null, null)
fun bad(): Int = if (next().text != null) next().text.length else 0
