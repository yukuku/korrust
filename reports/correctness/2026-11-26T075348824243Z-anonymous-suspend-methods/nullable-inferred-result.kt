import anonymouslibrary.StrictNames
fun maybe(): String? = null
fun use(): StrictNames = object : StrictNames() { override fun name() = maybe() }