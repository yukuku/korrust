package korrust.probes.models

fun formatInt(value: Int): String = "value=$value"
fun formatFlag(value: Boolean): String = "flag=$value"
fun formatText(value: String): String = "text=$value"
fun adjacent(value: Int): String = "$value$value"
fun dollarLiteral(value: Int): String = "literal=\$value"

fun escapedAndReal(value: Int): String = "literal=\$value real=$value \u0000\uD800"
fun unicodeDollar(value: Int): String = "\u0024value=$value"
