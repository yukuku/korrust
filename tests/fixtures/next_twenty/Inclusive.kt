package next.probes
import next.support.Tracker

fun inclusiveCount(start: Int, end: Int): Int { var n = 0; for (i in start..end) { n += 1 }; return n }
fun inclusiveFlow(): Int { var n = 0; for (i in 0..8) { if (i == 2) continue; if (i == 5) break; n += i }; return n }
fun inclusiveOnce(): Int { var n = 0; for (i in Tracker.bound(1)..Tracker.bound(3)) { n += i }; return n }
