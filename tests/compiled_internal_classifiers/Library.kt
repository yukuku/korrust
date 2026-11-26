package friends.library

internal class Plain(val value: Int) {
    fun bump(delta: Int = 2): Int = value + delta
}
internal data class Data(val value: Int = 4, val label: String? = null, val enabled: Boolean = false)
internal interface Counter { fun value(): Int; fun base(): Int = 31 }
class PublicImplementation : Counter { override fun value(): Int = 29 }
internal class Implementation(private val value: Int) : Counter {
    override fun value(): Int = value
}
internal object Singleton { fun value(delta: Int = 0): Int = 11 + delta }
internal enum class Choice {
    FIRST;
    fun value(): Int = 13
}
private class FileHidden
