package next.probes

fun decrement(value: Int): Int { var n = value; val before = n--; val after = --n; return before * 100 + after * 10 + n }
fun decrementWrap(value: Int): Int { var n = value; n--; return n }
