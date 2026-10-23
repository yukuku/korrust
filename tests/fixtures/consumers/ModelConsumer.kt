package korrust.consumers

import yuku.alkitab.model.SingleChapterVerses
import korrust.probes.models.RecordView

fun main() {
    check(SingleChapterVerses.EMPTY.verseCount == 0)
    check(SingleChapterVerses.EMPTY.getVerseNumberText(0) == "1")
    try { SingleChapterVerses.EMPTY.getVerse(2); error("expected exception") }
    catch (error: IllegalArgumentException) { check(error.message == "@@getVerse requesting verse_0=2 but verseCount=0") }
    val sized = object : SingleChapterVerses.WithTextSizeMult {
        override val verseCount: Int get() = 2
        override fun getVerse(verse_0: Int): String = "verse"
        override fun getTextSizeMult(verse_0: Int): Float = 1.25f
    }
    check(sized.getVerseNumberText(39) == "40")
    check(sized.getTextSizeMult(0) == 1.25f)
    check(RecordView.VACANT.itemCount == 7)
    check(RecordView.VACANT.label(0) == "7")
    val weighted = object : RecordView.WithWeight {
        override val itemCount: Int get() = 2
        override fun itemAt(position: Int): String = "record"
        override fun weight(position: Int): Float = -2.5f
    }
    check(weighted.label(0) == "\u0000二")
    check(weighted.weight(0) == -2.5f)
    println("model companion/default/nested metadata consumer OK")
}
