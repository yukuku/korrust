interface Api<C> { fun <T> count(vararg items:T):Int }
class Impl<C>:Api<C> { override fun <R> count(vararg items:R?):Int=items.size }