package twenty
import yuku.alkitab.base.audio.AudioLogEntry
interface MemberSurface {
    fun nullable(value: Int?): Int?
    fun logs(): List<AudioLogEntry>
}
