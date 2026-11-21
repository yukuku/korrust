package versesconsumer
import yuku.alkitab.base.verses.VersesUiModel
fun main() {
    val empty = VersesUiModel.EMPTY
    check(empty.textSizeMult == 1f)
    check(empty.dictionaryModeAris.isEmpty())
    check(!empty.isVerseNumberShown)
    println("OK")
}
