package next.probes

import next.support.Tracker
fun arraySum(values: IntArray): Int { var n = 0; for (v in values) { n += v }; return n }
fun arrayOnce(values: IntArray): Int { var n = 0; for (v in Tracker.array(values)) { n += v }; return n }
fun byteSum(values: ByteArray): Int { var n = 0; for (v in values) { n += v.toInt() }; return n }
fun charSum(values: CharArray): Int { var n = 0; for (v in values) { n += v.code }; return n }
