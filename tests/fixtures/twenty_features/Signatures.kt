package twenty
import yuku.alkitab.base.audio.AudioLogEntry
class Signatures(val optional: Int?, val logs: List<AudioLogEntry>) {
    fun nullable(value: String?): String? = value
    fun entries(value: List<AudioLogEntry>): List<AudioLogEntry> = value
    fun count(value: Int?): Int? = value
    val nullableList: List<Int?> get() = listOf<Int?>(optional)
}
