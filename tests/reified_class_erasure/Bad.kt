import library.classOf
fun bad(): Class<Set<String>> = classOf<List<String>>()
