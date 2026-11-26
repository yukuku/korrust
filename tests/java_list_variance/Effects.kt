import variance.Bags
fun fields(holder: Bags, value: MutableList<String>) {
    holder.field = value
    holder.value = value
}
fun readonly(value: List<String>): Int = Bags.accept(value)
fun projectedSame(value: MutableList<out String>): Int = Bags.same(value)
fun flexibleSame(): Int = Bags.same(Bags.strings())
fun nullable(): Int = Bags.nullable(null)
fun once(): Int = Bags.same(Bags.next())
fun inferred() = Bags.strings()
