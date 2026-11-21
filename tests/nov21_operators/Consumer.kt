package consumer
import updates.Counter
import updates.Holder
import updates.Extension
import updates.inc
import updates.dec
fun main() {
    var value = Counter(5)
    val old = value++
    val h = Holder(value)
    h.item += 2
    if (old.value != 5 || h.saved.value != 8) throw IllegalStateException("metadata")
    var extra = Extension(7)
    extra++
    --extra
    if (extra.value != 7) throw IllegalStateException("extension metadata")
    println("OK")
}
