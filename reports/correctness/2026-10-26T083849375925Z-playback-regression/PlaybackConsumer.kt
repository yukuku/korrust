import yuku.alkitab.base.audio.PlaybackState
import compatibility.Snapshot
import compatibility.nullableList

fun main() {
    val idle = PlaybackState.IDLE
    check(!idle.isActive && idle.logs.isEmpty() && idle.speed == 1.0f)
    check(idle.copy(bookId = 2).isActive)
    check(idle.copy() == idle && idle.component5() == -1)
    check(Snapshot.EMPTY.logs.isEmpty() && nullableList()!!.isEmpty())
    println("kotlin-consumer=pass")
}
