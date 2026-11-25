package genericargs
class Defaults<C>(val saved:C) {
    fun <T> fallback(value:T?=null):T?=value
    fun <T> repeat(first:T, second:T=first):T=second
    fun <T> classifier(ignored:T, selected:C=saved):C=selected
    fun <T> nullable(first:T, second:T?=null):T?=second
    fun <T> sequence(first:T, second:T=first, third:T=second):T=third
}
fun <T> Int.extension(value:T):T=value
fun sourceDefaults():String {
    val box=Defaults<String>("OK")
    if (1.extension<String>("OK") != "OK") return "extension"
    if (box.fallback<String>() != null) return "fallback"
    if (box.repeat("OK") != "OK") return "repeat"
    if (box.classifier(1) != "OK") return "classifier"
    if (box.nullable("OK") != null) return "nullable"
    if (box.sequence(third="OK", first="bad") != "OK") return "named"
    if (box.sequence("OK") != "OK") return "sequence"
    return "OK"
}
