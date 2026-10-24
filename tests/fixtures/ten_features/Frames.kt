package ten

import yuku.alkitab.model.SingleChapterVerses

class Frames(private val text: String?, val count: Long) : SingleChapterVerses {
    override val verseCount: Int get() = 1
    override fun getVerse(verse_0: Int): String = "${count + 1L}:${text ?: "absent"}"
    fun same(a: Long, b: Long, name: String): String = "${if (a > b) a else b}:${text ?: name}"
}
