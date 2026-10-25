package next.probes

fun doWhileSum(limit: Int): Int { var i = 0; var total = 0; do { i++; if (i == 2) continue; total += i } while (i < limit); return total }
fun doWhileBreak(): Int { var i = 0; do { i++; if (i == 3) break } while (true); return i }
