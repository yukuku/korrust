interface Label
interface Api<C> { fun <T:Label> count(vararg items:T):Int }
class Impl<C>:Api<C> { override fun <R> count(vararg items:R):Int=items.size }