package staticprobe
import kotlin.jvm.JvmField as Export
class Carrier(val value: Int)
class Cache {
    companion object {
        @Export val EMPTY = Carrier(4)
        @Export var boxed: Int? = null
        @Export val names: List<String> = listOf("a", "b")
        @Export val number: Long = 8L
        @Export val decimal: Double = 2.5
        fun getEMPTY(): String = "method"
    }
}
object Aliased { @Export var text: String = "alias" }
