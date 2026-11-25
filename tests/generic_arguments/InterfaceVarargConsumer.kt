import genericargs.InterfaceVarargApi
import genericargs.InterfaceVarargs
import genericargs.InterfaceLabel
class KotlinApi:InterfaceVarargApi<String> {
    override fun <R> count(vararg items:R):Int=items.size
    override fun <R:InterfaceLabel> label(vararg items:R):String=items[0].text()
    override fun <R> select(ignored:R, vararg items:String):String=items[0]
}
fun main() {
    check(genericargs.throughInterface(InterfaceVarargs<String>()) == "OK")
    check(genericargs.throughInterface(KotlinApi()) == "OK")
    println("OK")
}
