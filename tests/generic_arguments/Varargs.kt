package genericargs
interface Label { fun text():String }
class Word(val value:String):Label { override fun text():String=value }
class Varargs<C>(val saved:C) {
    fun <T> packed(vararg items:T):Any=items
    fun <T> first(vararg items:T):T=items[0]
    fun <T> choose(first:T, second:T=first, vararg items:T):T=if(items.size==0) second else items[0]
    fun <T> forward(first:T, vararg items:T):T=choose(first=first, items=items)
    fun <T> count(ignored:T, vararg items:T):Int {
        val alias=items
        var count=0
        for (item in alias) count=count+1
        return count
    }
    fun <T:Label> label(vararg items:T):String=items[0].text()
    fun <T> classifier(ignored:T, vararg items:C):C=items[0]
    fun <T> primitive(ignored:T, vararg items:Int):Int=items[0]
}
fun sourceVarargs():String {
    val box=Varargs<String>("saved")
    if (box.first("OK", "wrong") != "OK") return "first"
    if (box.first(7, 8) != 7) return "boxing"
    val values=Array<String>(2) { index -> if(index==0) "OK" else "wrong" }
    if (box.packed(*values) === values) return "spread-copy"
    if (box.first(*values) != "OK") return "spread"
    if (box.first(items=values) != "OK") return "named"
    if (box.choose("OK") != "OK") return "empty/default"
    if (box.choose(first="wrong", items=values) != "OK") return "named/default"
    if (box.forward("OK") != "OK") return "empty-forward"
    if (box.forward("wrong", *values) != "OK") return "forward"
    if (box.count("ignored", "A", "B") != 2) return "iterate"
    if (box.label(Word("OK")) != "OK") return "bound"
    if (box.classifier(1, "OK") != "OK") return "class"
    if (box.primitive("ignored", 7, 8) != 7) return "primitive"
    return "OK"
}
