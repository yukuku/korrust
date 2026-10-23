package korrust.consumers

import yuku.alkitab.model.SingleChapterVerses
import yuku.alkitab.yes2.Yes2SingleChapterVerses
import yuku.alkitab.yes2.util.unsignedIntBinarySearch

fun main() {
    check(SingleChapterVerses.EMPTY.verseCount == 0)
    check(SingleChapterVerses.EMPTY.getVerseNumberText(39) == "40")
    val chapter = Yes2SingleChapterVerses(arrayOf("verse"))
    check(chapter.verseCount == 1)
    check(chapter.getVerse(0) == "verse")
    check(chapter.getVerseNumberText(0) == "1")
    check(unsignedIntBinarySearch(intArrayOf(0, 1, -1), -1) == 2)
    val sized = object : SingleChapterVerses.WithTextSizeMult {
        override val verseCount: Int get() = 0
        override fun getVerse(verse_0: Int): String = "x"
        override fun getTextSizeMult(verse_0: Int): Float = 2.5f
    }
    check(sized.getTextSizeMult(0) == 2.5f)
    println("combined Model/Yes2 metadata consumer OK")
}
