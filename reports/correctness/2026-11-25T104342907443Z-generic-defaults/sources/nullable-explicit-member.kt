class Box<C> { fun <T> fallback(value:T?=null):T?=value }
fun bad(box:Box<String>?):String?=box.fallback<String>()