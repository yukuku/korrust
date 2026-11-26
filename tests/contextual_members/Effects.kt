import contextcalls.Item
import contextcalls.Leaf
import contextcalls.Service

class Holder : Service() {
    lateinit var result: Leaf
    fun fill(kind: Int) { result = lookup(kind) }
    fun implicit(kind: Int): Leaf = lookup(kind)
    fun throughThis(kind: Int): Leaf = this.lookup(kind)
    fun mixed(): Leaf = mixed(1)
    fun zero(): Leaf = lookup()
    fun overload(): Leaf = lookup("xxxx")
    fun local(kind: Int): Leaf {
        var value: Leaf = Leaf(-1)
        value = lookup(kind)
        return value
    }
    fun nullable(kind: Int): Leaf? = lookup(kind)
}
fun explicit(value: Service, kind: Int): Leaf = value.lookup(kind)
fun broad(value: Service): Item = value.lookup(1)
fun computed(): Leaf = Service.receiver().lookup(Service.argument())
fun assign(value: Service): Leaf {
    var result: Leaf = Leaf(-1)
    result = value.lookup(1)
    return result
}
fun <T : Item> parameterized(value: Service): T = value.lookup(1)
fun mixed(value: Service): Leaf = value.mixed(1)
