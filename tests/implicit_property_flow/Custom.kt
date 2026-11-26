class Row { val n: Int? get() = 3 }
fun Row.bad(): Int { if (n != null) return n; return 0 }
