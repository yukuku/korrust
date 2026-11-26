package app
import app.Store.next as increment
fun count(s: Store): Int = s.read(Store.next(2)) + increment(0)
class Child : Store() { fun own(): Int = read(4) }
