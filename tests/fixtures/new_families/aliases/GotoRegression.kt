package aliasregression

import yuku.alkitab.base.compose.goto.GotoTab
import yuku.alkitab.base.compose.goto.OnGotoFinished

fun main() {
    var actual = ""
    val finished: OnGotoFinished = { tab, book, chapter, verse -> actual = tab.name + ":" + tab.ordinal1Based + ":" + book + ":" + chapter + ":" + verse }
    finished(GotoTab.GRID, 8, 3, 5)
    check(actual == "GRID:3:8:3:5")
    println("OK")
}
