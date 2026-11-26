import variance.Bags
fun consume(): Boolean {
    val holder = Bags()
    fields(holder, mutableListOf("a"))
    val items = inferred()
    items.add("b")
    return holder.readSize() == 1 && readonly(listOf("a")) == 1 && items.size == 1 && nullable() == -1
}
