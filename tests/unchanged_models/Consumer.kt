package proof
import yuku.alkitab.base.audio.AudioLogEntry
import yuku.alkitab.base.audio.PlaybackState
import yuku.alkitab.datatransfer.model.Gid
import yuku.alkitab.datatransfer.model.Rpp
fun checkModels():String {
 val idle = PlaybackState.IDLE
 check(!idle.isPlaying && !idle.preparing && !idle.isActive)
 check(idle.versionId == "" && idle.audioId == "" && idle.bookId == -1)
 check(idle.chapter_1 == 0 && idle.verse_1 == 0 && idle.positionMs == 0L && idle.durationMs == 0L)
 check(idle.speed == 1.0f && idle.error == null && idle.logs.isEmpty())
 val logs = listOf(AudioLogEntry(123L, "connected"))
 val state = PlaybackState(true, false, "internal", "recording", 4, 5, 6, 7L, 8L, 1.25f, "retry", logs)
 check(state.isActive && state.component1() && state.component3() == "internal")
 check(state.component8() == 7L && state.component10() == 1.25f && state.component12() == logs)
 check(state == state.copy() && state.hashCode() == state.copy().hashCode())
 val changed = state.copy(bookId = -1, error = null)
 check(!changed.isActive && changed != state && changed.error == null && changed.logs === logs)
 check(state.toString() == "PlaybackState(isPlaying=true, preparing=false, versionId=internal, audioId=recording, bookId=4, chapter_1=5, verse_1=6, positionMs=7, durationMs=8, speed=1.25, error=retry, logs=[AudioLogEntry(timestampMs=123, message=connected)])")
 val rpp = Rpp(Gid("gid-1"), 9223372036854775806L, listOf(0, 4, -1))
 check(rpp.gid.value == "gid-1" && rpp.startTime == 9223372036854775806L && rpp.done == listOf(0, 4, -1))
 check(rpp.component1() == Gid("gid-1") && rpp.component2() == 9223372036854775806L && rpp.component3() == listOf(0, 4, -1))
 check(rpp == rpp.copy() && rpp.hashCode() == rpp.copy().hashCode())
 val later = rpp.copy(gid = Gid("gid-2"), startTime = -9L)
 check(later.gid.value == "gid-2" && later.startTime == -9L && later.done === rpp.done && later != rpp)
 check(rpp.toString() == "Rpp(gid=gid-1, startTime=9223372036854775806, done=[0, 4, -1])")
 return "OK"
}
