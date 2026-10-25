package next.probes

import next.support.Tracker
fun stringSum(value: String): Int { var n = 0; for (c in value) { n += c.code }; return n }
fun stringOnce(value: String): Int { var n = 0; for (c in Tracker.text(value)) { n += c.code }; return n }
