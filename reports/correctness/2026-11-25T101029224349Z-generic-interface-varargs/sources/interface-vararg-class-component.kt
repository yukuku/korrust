interface Api<C> { fun <T> pick(ignored:T, vararg items:C):C }
class Impl:Api<String> { override fun <R> pick(ignored:R, vararg items:Int):String="bad" }