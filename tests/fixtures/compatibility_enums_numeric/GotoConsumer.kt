package compatibility.enumsnumeric

import yuku.alkitab.base.compose.goto.GotoTab
import yuku.alkitab.base.compose.goto.OnGotoFinished

fun gotoAliasProbe(): String {
    var result: String = ""
    val finished: OnGotoFinished = { tab, bookId, chapter, verse -> result = tab.name + ":" + tab.ordinal1Based + ":" + bookId + ":" + chapter + ":" + verse }
    finished(GotoTab.GRID, 8, 3, 5)
    return result
}

fun gotoAliasConsumer(callback: OnGotoFinished) {
    callback(GotoTab.DIRECT, 2, 4, 6)
}

fun enumMetadataProbe(): String = Signal.RED.caption() + ":" + Signal.GREEN.doubled + ":" + Signal.entries.size

fun doubleMetadataProbe(value: Double?): Double? = DoubleBox(1.0, value).optional

fun main() {
    check(gotoAliasProbe() == "GRID:3:8:3:5")
    var value: Int = 0
    gotoAliasConsumer { tab, book, chapter, verse -> value = tab.ordinal1Based + book + chapter + verse }
    check(value == 14)
    check(enumMetadataProbe() == "red:1:4:2")
    check(doubleMetadataProbe(2.5) == 2.5)
    check(doubleMetadataProbe(null) == null)
    val record = DoubleRecord(1.5, null).copy(value = 2.5)
    check(record.value == 2.5 && record.optional == null)
    check(defaulted() == 1.5)
    check(DefaultBox().value == 2.5)
    check(counter(1.0)() == 1.5)
    check(constant == 1.25)
    println("kotlin-enums-numeric-ok")
}
