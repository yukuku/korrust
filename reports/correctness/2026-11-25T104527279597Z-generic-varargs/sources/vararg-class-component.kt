class Box<C> { fun <T> first(ignored:T, vararg items:C):C=items[0] }
fun bad():String=Box<String>().first(1,2)