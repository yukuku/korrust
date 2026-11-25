package genericargs
interface InterfaceLabel { fun text():String }
class InterfaceWord(val value:String):InterfaceLabel { override fun text():String=value }
interface InterfaceVarargApi<C> {
    fun <T> count(vararg items:T):Int
    fun <T:InterfaceLabel> label(vararg items:T):String
    fun <T> select(ignored:T, vararg items:C):C
}
class InterfaceVarargs<C>:InterfaceVarargApi<C> {
    override fun <R> count(vararg items:R):Int=items.size
    override fun <R:InterfaceLabel> label(vararg items:R):String=items[0].text()
    override fun <R> select(ignored:R, vararg items:C):C=items[0]
}
fun throughInterface(api:InterfaceVarargApi<String>):String {
    if(api.count("A", "B") != 2) return "count"
    if(api.count<String>() != 0) return "empty"
    if(api.label(InterfaceWord("OK")) != "OK") return "bound"
    if(api.select(1, "OK") != "OK") return "classifier"
    return "OK"
}
fun sourceInterfaceVarargs():String {
    val impl=InterfaceVarargs<String>()
    if(impl.count(1,2) != 2) return "concrete"
    return throughInterface(impl)
}
