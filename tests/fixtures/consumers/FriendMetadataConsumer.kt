package korrust.consumers

import yuku.alkitab.yes2.Yes2SingleChapterVerses
import korrust.probes.internalIdentity

fun main() {
    val backing = arrayOf("first", "last")
    val chapter = Yes2SingleChapterVerses(backing)
    check(chapter.verseCount == 2)
    check(chapter.getVerse(1) == "last")
    check(chapter.getVerseNumberText(0) == "1")
    backing[0] = "changed"
    check(chapter.getVerse(0) == "changed")
    check(internalIdentity("friend") == "friend")
    println("friend metadata consumer OK")
}
