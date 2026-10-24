package ten

import ten.trace.Trace

fun nullableIdentity(value: String?): String? = value
fun nullableLiteral(): String? = null
fun nullableEqual(a: String?): Boolean = a == null
fun nullableNotEqual(a: String?): Boolean = a != null
fun nullableReceiver(trace: Trace, value: String?): String? { trace.markReceiver(); return value }
fun nullConditionEqual(trace: Trace, value: String?): Boolean = (if (trace.firstCondition()) null else null) == nullableReceiver(trace, value)
fun nullConditionElvis(trace: Trace): String = (if (trace.firstCondition()) null else null) ?: trace.fallback()!!
fun safeRemove(trace: Trace, value: String?): String? = nullableReceiver(trace, value)?.removePrefix(trace.argument()!!)
fun safeChain(trace: Trace, value: String?): String? = nullableReceiver(trace, value)?.removePrefix("x")?.substring(0)
fun elvis(trace: Trace, value: String?): String = nullableReceiver(trace, value) ?: trace.fallback()!!
fun safeElvis(trace: Trace, value: String?): String = safeRemove(trace, value) ?: trace.fallback()!!
fun listSize(values: List<String>): Int = values.size
fun listGet(values: List<String>, index: Int): String = values[index]
fun nullableListGet(values: List<String?>, index: Int): String? = values[index]
fun listFallback(trace: Trace, values: List<String?>, index: Int): String = values[index] ?: trace.fallback()!!
fun stringWhen(value: String): Int = when (value) { "first", "second" -> 1; "\u0000\ud800" -> 2; else -> 3 }
fun stringWhenTrace(trace: Trace): String = when (trace.subject()!!) { trace.first()!!, trace.second()!! -> trace.matched()!!; else -> trace.fallback()!! }
fun booleanWhen(value: Boolean): String = when (value) { true -> "yes"; else -> "no" }
fun booleanWhenStatement(value: Boolean): Int { when (value) { true -> return 1; else -> return 0 } }
fun longWhen(value: Long): String = when (value) { Long.MIN_VALUE -> "minimum"; 9223372036854775807L -> "maximum"; else -> "other" }
fun subjectlessWhen(trace: Trace): String = when { trace.firstCondition() -> trace.first()!!; trace.secondCondition() -> trace.second()!!; else -> trace.fallback()!! }
fun subjectlessStatement(value: Int): Int { when { value < 0 -> return -1; value == 0 -> return 0; else -> return 1 } }
fun template(value: String?, number: Long): String = "before:${value ?: "<null>"}:${number + 1L}:${"{" + "}"}:after"
fun templateTrace(trace: Trace): String = "${trace.first()!!}|${trace.second()!!}|${trace.fallback()!!}"
fun wideAnswer(): Long = 9223372036854775807L
fun templateWideCall(): String = "answer=${wideAnswer()}"
fun wideCalls(): Long = wideAnswer() + wideAnswer()
fun templateEscaped(): String = "\${literal}:\u0000\ud800:${"\udfff"}"
fun nullableTemplate(value: String?): String = "${value}:${null}"
fun nullableConcat(trace: Trace, value: String?): String = "prefix:" + nullableReceiver(trace, value) + ":" + null
