import contextlib.Flag
import contextlib.Token
import kotlin.coroutines.CoroutineContext
fun combine(left: Token, right: Flag): CoroutineContext = left + right
fun named(left: Token, right: Flag): CoroutineContext = left.plus(right)
fun specific(left: Token, right: Token): Token = left + right
