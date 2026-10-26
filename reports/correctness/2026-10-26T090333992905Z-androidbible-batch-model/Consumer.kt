import yuku.alkitab.model.SingleChapterVerses
fun main() {
 check(SingleChapterVerses.EMPTY.verseCount == 0)
 val verses = object : SingleChapterVerses {
  override val verseCount: Int get() = 50
  override fun getVerse(verse_0: Int): String = "verse"
 }
 check(verses.getVerseNumberText(0) == "1")
 check(verses.getVerseNumberText(38) == "39")
 check(verses.getVerseNumberText(39) == "40")
 println("PASS")
}
