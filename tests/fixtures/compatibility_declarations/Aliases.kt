package compatibility.declarations

typealias Label = String
typealias Caption = Label
typealias IntTransform = (Int) -> Int
typealias IntCallback = IntTransform
class GotoTab(val ordinal1Based: Int)
typealias OnGotoFinished = (tab: GotoTab, bookId: Int, chapter_1: Int, verse_1: Int) -> Unit
private typealias HiddenLabel = String

fun label(value: Caption): Label = value
fun aliasApply(value: Int, action: IntCallback): Int = action(value)
fun privateAliasBody(value: String): String { val hidden: HiddenLabel = value; return hidden }
