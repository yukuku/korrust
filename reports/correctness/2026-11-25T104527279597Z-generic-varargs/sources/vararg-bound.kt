interface Label
class Box<C> { fun <T:Label> first(vararg items:T):T=items[0] }
fun bad():String=Box<Int>().first("bad")