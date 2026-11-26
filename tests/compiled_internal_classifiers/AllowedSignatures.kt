package friends.consumer
import friends.library.Plain

internal fun hiddenSignature(value: Plain): Plain = value
internal class InternalWrapper(val value: Plain) {
    fun echo(other: Plain): Plain = other
}
class PrivateConstructor private constructor(value: Plain) {
    private val stored: Plain = value
}
class InternalConstructor internal constructor(private val stored: Plain)
