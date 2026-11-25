fun <T> mutate(items:Array<T>, value:T) { items[0]=value }
class Box<C> { fun <T> bad(x:T, vararg items:T) { mutate(items,x) } }