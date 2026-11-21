class Good
operator fun Good?.inc(): Good = this!!
fun use() { var value: Good? = null; value++ }
