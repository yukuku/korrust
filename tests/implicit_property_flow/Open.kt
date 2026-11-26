open class Row { open val n: Int? = 3 }
fun Row.bad(): Int { if (n != null) return n; return 0 }
