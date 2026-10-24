package twenty
import yuku.alkitab.base.audio.AudioLogEntry
import twenty.MemberSurface
class MemberImplementation(val entries: List<AudioLogEntry>) : MemberSurface {
    override fun nullable(value: Int?): Int? = value
    override fun logs(): List<AudioLogEntry> = entries
}
