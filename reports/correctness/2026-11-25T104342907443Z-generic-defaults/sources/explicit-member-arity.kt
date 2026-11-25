class Box<C> { fun <T> fallback(value:T?=null):T?=value }
fun bad():String?=Box<String>().fallback<String,Int>()