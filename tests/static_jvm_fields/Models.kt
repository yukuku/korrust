package staticprobe
object Trace { var text: String = ""; fun note(value: String): String { text += value; return value } }
class Host {
    companion object {
        @JvmField val first: String = Trace.note("A")
        val ordinary: String = Trace.note("B")
        @JvmField var mutable: String? = Trace.note("C")
        init { Trace.note("D") }
        fun own(): String = first + ordinary + mutable
        fun deferred(): () -> String = { first }
    }
}
object Global {
    @JvmField val value: String = Trace.note("E")
    @JvmField var number: Int = 7
    val ordinary: String = Trace.note("F")
    fun own(): String = value + ordinary + number
}
