package matcher
open class Scope {
    var counter = 0
    fun defaultNumber(): Long { counter++; return 7L }
    inline fun <reified T : Any> check(value: T, number: Long = defaultNumber(), invert: Boolean = false): Boolean =
        (T::class.java.isInstance(value) && number == 7L) != invert
}
