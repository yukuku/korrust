package ten.external

import ten.*
import yuku.alkitab.base.audio.AudioLogEntry

fun invalid() {
    AudioLogEntry(0L, null)
    listGet(listOf<String?>(null), 0)
    val record = ScalarRecord(1, 2, 'x', true, 3L, "text")
    record.copy(message = null)
    record.message = "changed"
    val fields = PublicFields(1, 2L, "label", true, "title", null, listOf(null), 3)
    fields.label = "changed"
    val text: String = fields.optional
}
