package contextlib
import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext
class Token : AbstractCoroutineContextElement(Key) {
    companion object Key : CoroutineContext.Key<Token>
    operator fun plus(other: Token): Token = other
}
class Flag : AbstractCoroutineContextElement(Key) {
    companion object Key : CoroutineContext.Key<Flag>
}
