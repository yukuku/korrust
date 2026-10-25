package next.probes

import next.support.Tracker
fun identical(a: Any?, b: Any?): Boolean = a === b
fun different(a: Any?, b: Any?): Boolean = a !== b
fun identityOrder(a: Any?, b: Any?): Boolean = Tracker.objectValue(a) === Tracker.objectValue(b)
