package twenty
import yuku.alkitab.base.audio.AudioLogEntry
data class RichRecord(val optional: Int?, val text: String?, val score: Float, val logs: List<AudioLogEntry>) {
    val active: Boolean get() = optional != null
    fun summary(): String = "${text ?: "missing"}:${logs.size}"
}
