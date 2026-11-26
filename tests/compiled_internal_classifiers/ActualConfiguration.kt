package yuku.alkitab.base.appwidget
fun actualConfiguration(): Int {
    val original = DailyVerseConfiguration(textSize = 21)
    val changed = original.copy(transparency = 3)
    return changed.textSize + changed.transparency + if (changed.isTransparent) 1 else 0
}
