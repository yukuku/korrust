package next.probes

import next.support.Tracker
fun equalObjects(a: Any?, b: Any?): Boolean = a == b
fun unequalObjects(a: Any?, b: Any?): Boolean = a != b
fun equalStrings(a: String?, b: String?): Boolean = a == b
fun equalityOrder(a: Any?, b: Any?): Boolean = Tracker.objectValue(a) == Tracker.objectValue(b)
