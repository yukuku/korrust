package ten.consumer

import ten.*
import ten.trace.Trace
import yuku.alkitab.base.audio.AudioLogEntry
import yuku.alkitab.base.dialog.VersesDialogNormalVerses
import yuku.alkitab.base.dialog.XrefDialogVerses
import yuku.alkitab.model.SingleChapterVerses

private fun observe(id: String, action: () -> Unit) {
    action()
    println("{\"id\":\"$id\",\"outcome\":\"pass\"}")
}

fun main() {
    val trace = Trace()
    observe("long_metadata") {
        check(longAdd(Long.MAX_VALUE, 1L) == Long.MIN_VALUE)
        check(longSlots(Long.MIN_VALUE, true, Long.MAX_VALUE, -1) == -2L)
        check(longText(Long.MIN_VALUE) == "-9223372036854775808")
    }
    observe("public_val_metadata") {
        val texts = mutableListOf<String?>(null, "text")
        val fields = PublicFields(1, 2L, "label", true, "title", null, texts, 3)
        val optional: String? = fields.optional
        val list: List<String?> = fields.texts
        check(optional == null && list === texts)
        check(fields.count == 1 && fields.timestamp == 2L)
        check(fields.label == "label" && fields.isReady && fields.isTitle == "title")
    }
    observe("data_metadata_copy_destructure") {
        val value = ScalarRecord(count = 1, tiny = 2, character = '\ud800', isReady = true, timestamp = Long.MIN_VALUE, message = "\u0000\udfff")
        val (count, tiny, character, isReady, timestamp, message) = value
        check(count == 1 && tiny == 2.toByte() && character == '\ud800' && isReady)
        check(timestamp == Long.MIN_VALUE && message == "\u0000\udfff")
        check(value.copy() == value && value.copy() !== value)
        val changed = value.copy(timestamp = Long.MAX_VALUE, message = "changed")
        check(changed.timestamp == Long.MAX_VALUE && changed.message == "changed" && changed.count == 1)
        val audio = AudioLogEntry(timestampMs = Long.MIN_VALUE, message = "audio")
        val (time, text) = audio
        check(time == Long.MIN_VALUE && text == "audio")
        check(audio.copy(message = "changed") == AudioLogEntry(Long.MIN_VALUE, "changed"))
        check(audio.copy() == audio)
        val large = ThirtyTwo(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32)
        check(large.copy() == large && large.copy(p32 = -7).p32 == -7)
        check(large.copy(p1 = -9).p32 == 32 && large.copy(p32 = -7).p1 == 1)
    }
    observe("nullable_metadata") {
        val output: String? = nullableIdentity(null)
        check(output == null && nullableLiteral() == null)
        check(nullableIdentity("text") == "text")
    }
    observe("safe_call_metadata") {
        val output: String? = safeRemove(trace, null)
        check(output == null && safeChain(trace, "xtext") == "text")
    }
    observe("elvis_metadata") {
        val text: String = elvis(trace, null)
        check(text == "fallback" && safeElvis(trace, "xtext") == "text")
    }
    observe("list_metadata_and_internal_friends") {
        val values = mutableListOf("first", "\u0000\ud800")
        val nullableValues = mutableListOf<String?>(null, "second")
        val normal: SingleChapterVerses = VersesDialogNormalVerses(values, listOf("1", "2"))
        val xref: SingleChapterVerses = XrefDialogVerses("missing", nullableValues, listOf("a", "b"))
        check(normal.verseCount == 2 && normal.getVerse(1) == "\u0000\ud800")
        check(xref.getVerse(0) == "missing" && xref.getVerseNumberText(1) == "b")
        val nullable: String? = nullableListGet(nullableValues, 0)
        val nonnull: String = listGet(values, 0)
        check(nullable == null && nonnull == "first" && listSize(values) == 2)
        nullableValues[0] = "changed"
        check(xref.getVerse(0) == "changed")
    }
    observe("when_subject_metadata") {
        check(stringWhen("first") == 1 && stringWhen("other") == 3)
        check(booleanWhen(true) == "yes" && booleanWhen(false) == "no")
    }
    observe("subjectless_metadata") {
        check(subjectlessStatement(Int.MIN_VALUE) == -1)
        check(subjectlessStatement(0) == 0 && subjectlessStatement(Int.MAX_VALUE) == 1)
    }
    observe("template_metadata") {
        check(template(null, Long.MAX_VALUE) == "before:<null>:-9223372036854775808:{}:after")
        check(templateEscaped() == "\${literal}:\u0000\ud800:\udfff")
        check(nullableTemplate(null) == "null:null" && nullableConcat(trace, null) == "prefix:null:null")
    }
}
