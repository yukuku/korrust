package next.probes

import next.support.Tracker
import next.support.Words
fun listSum(values: List<String>): Int { var n = 0; for (v in values) { n += v.length }; return n }
fun listOnce(values: List<String>): Int { var n = 0; for (v in Tracker.list(values)) { n += v.length }; return n }
fun listMutation(values: MutableList<String>): Int { var n = 0; for (v in values) { n += v.length; values.add("x") }; return n }
fun longListSum(values: List<Long>): Long { var n = 0L; for (v in values) { n += v }; return n }
fun nullableListSum(values: List<String?>): Int { var n = 0; for (v in values) { n += v?.length ?: 0 }; return n }
fun iterableSum(values: Iterable<String>): Int { var n = 0; for (v in values) { n += v.length }; return n }
fun nullableLongIterableSum(values: Iterable<Long?>): Long { var n = 0L; for (v in values) { n += v ?: 0L }; return n }
fun customIterableSum(values: Words): Int { var n = 0; for (v in values) { n += v.length }; return n }
