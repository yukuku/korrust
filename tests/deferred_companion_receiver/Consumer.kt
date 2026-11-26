import numbers.Holder
import numbers.plusAssign
import numbers.runCallback
class Host {
    private companion object { fun unrelated() = 9 }
    fun compute(): Int {
        val h = Holder(4)
        runCallback { h += 3 }
        return h.value
    }
}
