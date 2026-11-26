import scoped.deferred
fun wrong(value: String): Int { value.deferred { return 7 } }
