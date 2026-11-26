package fields

fun word(source: Fields): String? = source.words[1]
fun count(source: Fields): Int = source.words.toList().size
fun matrix(source: Fields): Int = source.matrix[0][1]
fun peer(source: Fields): Fields = source.peers[0]
fun replace(source: Fields, values: Array<String>) { source.words = values }
fun nested(): String = Fields.table[0][0]
