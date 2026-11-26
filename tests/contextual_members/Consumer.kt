import contextcalls.Leaf
import contextcalls.Service
fun consume(): Boolean {
    val holder = Holder()
    holder.fill(1)
    return holder.result.code == 1 && holder.implicit(3).code == 3 && explicit(Service(), 4).code == 4 && parameterized<Leaf>(Service()).code == 1
}
