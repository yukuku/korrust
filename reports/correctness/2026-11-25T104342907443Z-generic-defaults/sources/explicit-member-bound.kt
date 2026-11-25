interface Label
class Box<C> { fun <T:Label> fallback(value:T?=null):T?=value }
fun bad():String?=Box<String>().fallback<String>()