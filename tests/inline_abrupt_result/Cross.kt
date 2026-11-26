import scoped.cross
fun wrong(value: String): Int { value.cross { return 7 } }
