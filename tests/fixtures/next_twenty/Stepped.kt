package next.probes

fun steppedSum(start: Int, end: Int, stride: Int): Int { var n = 0; for (i in start..end step stride) { n += i }; return n }
fun descendingStepSum(start: Int, end: Int, stride: Int): Int { var n = 0; for (i in start downTo end step stride) { n += i }; return n }
fun progressionReuse(): Int { val values = 1..5 step 2; var total = 0; for (i in values) { total += i }; for (i in values) { total += i }; return total }
fun localProgressionNames(): Int { val step = 2; val downTo = 7; var total = downTo; for (i in 1..5 step step) { total += i }; return total }
