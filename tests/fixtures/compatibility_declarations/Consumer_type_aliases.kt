package compatibility.declarations.consumer
import compatibility.declarations.*
fun main() {
    val name: Caption = "caption"
    val transform: IntCallback = { value -> value * 2 }
    check(label(name) == "caption" && aliasApply(6, transform) == 12)
    check(chainedAlias("chain") == "chain" && callbackAlias(7) == 12)
    var finished = 0
    val callback: OnGotoFinished = { tab, book, chapter, verse -> finished = tab.ordinal1Based + book + chapter + verse }
    invokeGoto(callback)
    check(finished == 10)
    println("OK")
}
