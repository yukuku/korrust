package next.probes

import next.support.Tracker
fun safeString(value: Any?): String? = value as? String
fun safeOnce(value: Any?): String? = Tracker.objectValue(value) as? String
fun safeInt(value: Any?): Int? = value as? Int
