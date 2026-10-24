package outside

import yuku.alkitab.base.storage.InternalSingleChapterVerses
import yuku.alkitab.base.storage.VerseTextDecoder

fun invalid(decoder: VerseTextDecoder) {
    decoder.separateIntoVerses(null, false)
    decoder.makeIntoSingleString(null, true)
    InternalSingleChapterVerses(arrayOf<String?>(null))
}
