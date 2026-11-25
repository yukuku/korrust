package capturedvarargs
interface Label { fun text():String }
class Named(val value:String):Label { override fun text():String=value }
class Box<C>(val seed:C) {
 fun <T> first(vararg values:T):()->T = { values[0] }
 fun <T:Label> bounded(vararg values:T):()->String = { values[0].text() }
 fun <T> elements(ignored:T,vararg values:C):()->C = { values[0] }
 fun <T> size(vararg values:T):()->Int = { values.size }
}
