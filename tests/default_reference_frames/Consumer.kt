class Holder(val item: Nodes.Base = Nodes.Derived())
fun take(item: Nodes.Base = Nodes.Derived()): Int = item.n()
fun both(): Int = Holder().item.n() + take()
