package korrust.consumers

import yuku.alkitab.io.Utf8Decoder
import yuku.alkitab.model.SingleChapterVerses
import yuku.alkitab.yes2.Yes2SingleChapterVerses
import yuku.alkitab.yes2.util.unsignedIntBinarySearch

fun main() {
    val bytes = "İ ΟΣ 𐐀".toByteArray()
    check(Utf8Decoder.toString(bytes) == "İ ΟΣ 𐐀")
    check(Utf8Decoder.toStringLowerCase(bytes) == "i οσ 𐐨")
    check(Utf8Decoder.toString(bytes, 0, bytes.size) == "İ ΟΣ 𐐀")
    check(SingleChapterVerses.EMPTY.verseCount == 0)
    val chapter = Yes2SingleChapterVerses(arrayOf(Utf8Decoder.toString(bytes)))
    check(chapter.getVerse(0) == "İ ΟΣ 𐐀")
    check(chapter.getVerseNumberText(0) == "1")
    check(unsignedIntBinarySearch(intArrayOf(0, 1, -1), -1) == 2)
    println("combined Model/Io/Yes2 Kotlin metadata consumer OK")
}
