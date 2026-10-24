package simple.consumer

import simple.mutable.MutableInherited
import simple.mutable.MutableOverride
import simple.mutable.MutableSurface
import simple.mutable.ReadOnlyBase
import yuku.alkitab.base.storage.InternalSingleChapterVerses
import yuku.alkitab.base.storage.VerseTextDecoder
import yuku.alkitab.base.widget.ReaderGestureActions
import yuku.alkitab.model.SingleChapterVerses
import yuku.alkitab.songs.SongTextZoomable
import yuku.alkitab.yes1.Yes1SingleChapterVerses

private class Zoom(override var songTextZoomPercent: Int) : SongTextZoomable

private class Mutable : MutableInherited, MutableSurface.Nested, MutableOverride {
    override var count: Int = 0
    override var tiny: Byte = 0
    override var character: Char = '\u0000'
    override var isReady: Boolean = false
    override var amount: Float = 0.0f
    override var label: String = ""
    override var isTitle: String = ""
    override var ints: IntArray = intArrayOf()
    override var bytes: ByteArray = byteArrayOf()
    override var chars: CharArray = charArrayOf()
    override var lines: Array<String> = arrayOf()
    override var item: Int = 0
    override var value: Int = 0
}

private class Decoder : VerseTextDecoder {
    override fun separateIntoVerses(ba: ByteArray, lowercased: Boolean): Array<String> =
        ba.map { if (lowercased) it.toString() else "[$it]" }.toTypedArray()

    override fun makeIntoSingleString(ba: ByteArray, lowercased: Boolean): String =
        separateIntoVerses(ba, lowercased).joinToString("|")
}

private class Actions : ReaderGestureActions {
    val calls = mutableListOf<String>()
    override fun onFloaterAriSelected(ari: Int) { calls.add("ari:$ari") }
    override fun goToPreviousChapter() { calls.add("previous") }
    override fun goToNextChapter() { calls.add("next") }
    override fun applyPreferences() { calls.add("preferences") }
    override fun setFullScreenWithDrawerHandle(yes: Boolean) { calls.add("full:$yes") }
}

private fun observe(id: String, action: () -> Unit) {
    action()
    println("{\"id\":\"$id\",\"outcome\":\"pass\"}")
}

fun main() {
    observe("song_mutable_int_override") {
        val zoom: SongTextZoomable = Zoom(100)
        check(zoom.songTextZoomPercent == 100)
        for (value in intArrayOf(Int.MIN_VALUE, -1, 0, 100, Int.MAX_VALUE)) {
            zoom.songTextZoomPercent = value
            check(zoom.songTextZoomPercent == value)
        }
    }
    observe("mutable_primitives_and_is_names") {
        val mutable: MutableSurface = Mutable()
        mutable.count = Int.MIN_VALUE
        mutable.tiny = Byte.MIN_VALUE
        mutable.character = '\ud800'
        mutable.isReady = true
        mutable.label = "A\u0000\ud800\udfff"
        mutable.isTitle = "different"
        check(mutable.count == Int.MIN_VALUE && mutable.tiny == Byte.MIN_VALUE)
        check(mutable.character == '\ud800' && mutable.isReady)
        check(mutable.label == "A\u0000\ud800\udfff" && mutable.isTitle == "different")
        for (bits in intArrayOf(0, Int.MIN_VALUE, 0x7f800000, 0x7fc01234)) {
            mutable.amount = Float.fromBits(bits)
            check(mutable.amount.toRawBits() == bits)
        }
    }
    observe("mutable_array_types_aliasing") {
        val mutable: MutableSurface = Mutable()
        val ints = intArrayOf(Int.MIN_VALUE, 0, Int.MAX_VALUE)
        val bytes = byteArrayOf(Byte.MIN_VALUE, -1, 0, Byte.MAX_VALUE)
        val chars = charArrayOf('\u0000', '\ud800', '\udfff', '\uffff')
        val lines = arrayOf("", "\u0000", "\ud800", "\ud83d\ude00")
        mutable.ints = ints
        mutable.bytes = bytes
        mutable.chars = chars
        mutable.lines = lines
        check(mutable.ints === ints && mutable.bytes === bytes && mutable.chars === chars && mutable.lines === lines)
        mutable.lines[0] = "changed"
        check(lines[0] == "changed")
    }
    observe("nested_and_inherited_mutable_properties") {
        val implementation = Mutable()
        val inherited: MutableInherited = implementation
        val parent: MutableSurface = implementation
        inherited.count = Int.MAX_VALUE
        check(parent.count == Int.MAX_VALUE)
        parent.label = "inherited"
        check(inherited.readCount() == Int.MAX_VALUE && inherited.readLabel() == "inherited")
        val nested: MutableSurface.Nested = implementation
        nested.item = -23
        check(implementation.item == -23)
        val writable: MutableOverride = implementation
        val readOnly: ReadOnlyBase = implementation
        writable.value = 91
        check(readOnly.value == 91)
    }
    observe("decoder_byte_array_string_array_contract") {
        val decoder: VerseTextDecoder = Decoder()
        check(decoder.separateIntoVerses(byteArrayOf(), false).isEmpty())
        val bytes = byteArrayOf(Byte.MIN_VALUE, -1, 0, Byte.MAX_VALUE)
        check(decoder.separateIntoVerses(bytes, false).contentEquals(arrayOf("[-128]", "[-1]", "[0]", "[127]")))
        check(decoder.makeIntoSingleString(bytes, true) == "-128|-1|0|127")
    }
    observe("reader_action_interface_dispatch") {
        val actions = Actions()
        val receiver: ReaderGestureActions = actions
        receiver.onFloaterAriSelected(Int.MIN_VALUE)
        receiver.goToNextChapter()
        receiver.goToPreviousChapter()
        receiver.applyPreferences()
        receiver.setFullScreenWithDrawerHandle(true)
        receiver.setFullScreenWithDrawerHandle(false)
        check(actions.calls == listOf("ari:-2147483648", "next", "previous", "preferences", "full:true", "full:false"))
    }
    observe("chapter_classes_and_internal_friend_metadata") {
        val verses = arrayOf("first", "\u0000\ud800", "\ud83d\ude00")
        val chapters: List<SingleChapterVerses> = listOf(InternalSingleChapterVerses(verses), Yes1SingleChapterVerses(verses))
        for (chapter in chapters) {
            check(chapter.verseCount == 3)
            check(chapter.getVerse(1) == "\u0000\ud800")
            check(chapter.getVerseNumberText(-1) == "0")
            check(chapter.getVerseNumberText(38) == "39")
            check(chapter.getVerseNumberText(39) == "40")
            verses[0] = "mutated"
            check(chapter.getVerse(0) == "mutated")
        }
        check(SingleChapterVerses.EMPTY.verseCount == 0)
    }
}
