package constantprobe

const val TOP = 5

object Settings {
    private const val PRIVATE = -1
    internal const val INTERNAL = 2L * 1024L
    public const val LABEL: String = "settings"
    const val TOTAL = Host.BASE + TOP
    val runtime: Int = TOTAL + PRIVATE
    fun read(): Int = runtime + PRIVATE
}

class Host {
    val initial: Int = BASE
    val inferred = BASE
    fun inferredRead() = BASE
    fun read(): Int = BASE + PRIVATE
    fun shadow(BASE: Int): Int = BASE + 1
    fun localShadow(): Int { val BASE = 7; return BASE }
    companion object {
        private const val PRIVATE = -2
        public const val BASE = 20 + TOP
        internal const val TIMEOUT = 200L
        const val LABEL = "host" + Settings.LABEL
        const val FLAG: Boolean = true
        const val LETTER: Char = 'x'
        const val FLOAT: Float = -1.25F
        const val DOUBLE: Double = 2.5
        const val LARGE: Long = 9223372036854775807L
        val runtime: Int = BASE + PRIVATE
        fun read(): Int = runtime + PRIVATE
        fun shadow(BASE: Int): Int = BASE + 2
        fun inferredRead() = BASE
    }
}

interface Contract {
    fun instanceRead(): Int = LIMIT + PRIVATE
    companion object {
        private const val PRIVATE = -3
        const val LIMIT = Host.BASE + 1
        const val LABEL = Host.LABEL
        fun read(): Int = LIMIT + PRIVATE + 3
    }
}

class Implementation : Contract

enum class Choice {
    FIRST;
    companion object {
        const val CODE = 27
        fun read(): Int = CODE
    }
}

object ReceiverTrace {
    var count: Int = 0
    fun touch(): Int { count = count + 1; return count }
}
class Lazy {
    companion object {
        const val VALUE = 4
        val initialized: Int = ReceiverTrace.touch()
    }
}
fun qualifiedRead(): Int = Lazy.Companion.VALUE
fun effectfulReceiver(): Lazy.Companion { ReceiverTrace.count = ReceiverTrace.count + 10; return Lazy.Companion }
fun effectfulRead(): Int = effectfulReceiver().VALUE
