import kotlin.jvm.Volatile
import kotlin.jvm.Volatile as Shared

class Storage {
    @Volatile var count: Int = 1
    @field:Shared var text: String? = null
    companion object {
        @Volatile private var instance: Storage? = null
        @field:Volatile var published: Long = 0
        @JvmStatic fun exchange(value: Storage?): Storage? {
            val previous = instance
            instance = value
            return previous
        }
    }
}
object State { @Shared var flag: Boolean = false }
@Volatile var top: Long = 0
class Counter(@field:Volatile var value: Int)
