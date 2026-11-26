class Row(var n: Int?)
fun Row.bad(): Int { if (n != null) return n; return 0 }
