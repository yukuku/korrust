class Box<C> { fun <T> first(vararg items:T):T=items[0] }
fun bad(x:IntArray):Int=Box<String>().first(*x)