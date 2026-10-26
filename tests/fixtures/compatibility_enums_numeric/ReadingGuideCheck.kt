package compatibility.enumsnumeric

import yuku.alkitab.base.verses.ReadingGuide
import yuku.alkitab.base.verses.ReadingGuideMode
import yuku.alkitab.base.util.ReadingRange

fun main() {
    check(ReadingGuideMode.fromPreference("off") == ReadingGuideMode.OFF)
    check(ReadingGuideMode.fromPreference("line") == ReadingGuideMode.LINE)
    check(ReadingGuideMode.fromPreference(null) == ReadingGuideMode.OFF)
    check(ReadingGuideMode.fromPreference("unknown") == ReadingGuideMode.OFF)
    val guide = ReadingGuide(ReadingGuideMode.LINE, listOf(ReadingRange(3, 5), ReadingRange(9, 10)))
    check(guide.includes(3) && guide.includes(4) && guide.includes(5))
    check(!guide.includes(2) && !guide.includes(6))
    check(guide.startsAt(3) && !guide.startsAt(4))
    check(guide.endsAt(5) && !guide.endsAt(4))
    check(ReadingGuide.NONE.mode == ReadingGuideMode.OFF && ReadingGuide.NONE.ranges.isEmpty())
    println("reading-guide-selected-jvm-ok")
}
