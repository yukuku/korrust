package next.probes

fun smartLength(value: Any?): Int { if (value is String) return value.length; return -1 }
fun smartElse(value: Any?): Int { if (value !is String) { return -1 } else { return value.length } }
fun smartEarly(value: Any?): Int { if (value !is String) return -1; return value.length }
