package nov25.moduleconsumer

import yuku.alkitab.io.Utf8Decoder
import yuku.alkitab.model.SingleChapterVerses
import yuku.alkitab.yes2.Yes2SingleChapterVerses

fun chapter(bytes: ByteArray, isLowercased: Boolean): SingleChapterVerses {
    val text = if (isLowercased) Utf8Decoder.toStringLowerCase(bytes) else Utf8Decoder.toString(bytes)
    return Yes2SingleChapterVerses(arrayOf(text))
}

fun verify() {
    val text = "İ ΟΣ 𐐀"
    val bytes = text.toByteArray()
    check(Utf8Decoder.toString(bytes) == text)
    check(Utf8Decoder.toStringLowerCase(bytes) == "i οσ 𐐨")
    check(Utf8Decoder.toString(bytes, 0, bytes.size) == text)
    val values = arrayOf(Utf8Decoder.toString(bytes), "second")
    val actual: SingleChapterVerses = Yes2SingleChapterVerses(values)
    check(actual.verseCount == 2)
    check(actual.getVerse(0) == text)
    check(actual.getVerseNumberText(0) == "1")
    check(actual.getVerseNumberText(38) == "39")
    check(actual.getVerseNumberText(39) == "40")
    values[1] = "changed"
    check(actual.getVerse(1) == "changed")
    check(SingleChapterVerses.EMPTY.verseCount == 0)
    check(SingleChapterVerses.EMPTY.getVerseNumberText(0) == "1")
    try {
        SingleChapterVerses.EMPTY.getVerse(0)
        error("Empty chapter accepted a verse")
    } catch (expected: IllegalArgumentException) {
        check(expected.message == "@@getVerse requesting verse_0=0 but verseCount=0")
    }
    val styled = object : SingleChapterVerses.WithTextSizeMult {
        override val verseCount: Int get() = 1
        override fun getVerse(verse_0: Int): String = "styled"
        override fun getTextSizeMult(verse_0: Int): Float = 2.5f
    }
    check(styled.getTextSizeMult(0) == 2.5f)
    check(styled.getVerseNumberText(0) == "1")
    check(chapter(bytes, false).getVerse(0) == text)
    check(chapter(bytes, true).getVerse(0) == "i οσ 𐐨")
}
