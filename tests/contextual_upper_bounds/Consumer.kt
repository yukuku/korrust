fun wide(id: Int): Any? = Nodes.find(id)
fun narrow(): Nodes.Leaf = Nodes.find(1)
fun asArgument(): Boolean = Nodes.same(Nodes.first, Nodes.find(1))
