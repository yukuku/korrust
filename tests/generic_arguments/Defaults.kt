package genericargs
class Defaults<C>(val saved:C) {
    fun <T> repeat(first:T, second:T=first):T=second
    fun <T> classifier(ignored:T, selected:C=saved):C=selected
    fun <T> nullable(first:T, second:T?=null):T?=second
    fun <T> sequence(first:T, second:T=first, third:T=second):T=third
}
fun sourceDefaults():String {
    val box=Defaults<String>("OK")
    if (box.repeat("OK") != "OK") return "repeat"
    if (box.classifier(1) != "OK") return "classifier"
    if (box.nullable("OK") != null) return "nullable"
    if (box.sequence(third="OK", first="bad") != "OK") return "named"
    if (box.sequence("OK") != "OK") return "sequence"
    return "OK"
}
