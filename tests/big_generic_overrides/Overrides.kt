package genericdispatch

interface Label { fun text():String }
class Named(val textValue:String):Label { override fun text():String=textValue }
interface Api<C> {
 fun <T> echo(value:T):T
 fun <T:Label> bounded(value:T):T
 fun <T> choose(value:T):C
 fun <T> take(seed:C,value:T):T
}
class Impl<C>(val seed:C):Api<C> {
 override fun <R> echo(value:R):R=value
 override fun <R:Label> bounded(value:R):R=value
 override fun <R> choose(value:R):C=seed
 override fun <R> take(seed:C,value:R):R=value
}
class TextApi:Api<String> {
 override fun <R> echo(value:R):R=value
 override fun <R:Label> bounded(value:R):R=value
 override fun <R> choose(value:R):String="class"
 override fun <R> take(seed:String,value:R):R=value
}
interface PlainApi { fun <T> echo(value:T):T }
object Singleton:PlainApi { override fun <R> echo(value:R):R=value }
open class Base<C>(val seed:C) {
 open fun <T> echo(value:T):T=value
 open fun <T> choose(value:T):C=seed
}
class Child:Base<String>("parent") {
 override fun <R> echo(value:R):R=value
 override fun <R> choose(value:R):String="child"
}
interface Refined<C>:Api<C> { override fun <R> echo(value:R):R }
fun through(api:Api<String>):String = api.choose(7) + api.take("seed", "!")

class IntApi:Api<Int> {
 override fun <R> echo(value:R):R=value
 override fun <R:Label> bounded(value:R):R=value
 override fun <R> choose(value:R):Int=17
 override fun <R> take(seed:Int,value:R):R=value
}
