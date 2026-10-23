package korrust.consumers

import yuku.alkitab.yes2.Yes2SingleChapterVerses
import yuku.alkitab.yes2.util.unsignedIntBinarySearch

fun main() {
    val source = arrayOf("one", "two")
    val chapter = Yes2SingleChapterVerses(source)
    check(chapter.verseCount == 2)
    check(chapter.getVerse(1) == "two")
    check(chapter.getVerseNumberText(39) == "40")
    source[0] = "mutated"
    check(chapter.getVerse(0) == "mutated")
    check(unsignedIntBinarySearch(intArrayOf(0, 1, -1), -1) == 2)
    println("module metadata consumer OK")
}
