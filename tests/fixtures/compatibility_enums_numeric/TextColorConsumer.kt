package compatibility.enumsnumeric

import yuku.alkitab.base.util.TextColorUtil

fun main() {
    check(TextColorUtil.getSearchKeywordByBrightness(0.5f) == 0xff00c853.toInt())
    check(TextColorUtil.getForCheckedVerse(0xff000000.toInt()) == 0xffffffff.toInt())
    check(TextColorUtil.getForCheckedVerse(0xffffffff.toInt()) == 0xff000000.toInt())
    println("kotlin-text-color-ok")
}
