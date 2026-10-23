package korrust.consumers

import korrust.probes.add
import korrust.probes.identity
import korrust.probes.classes.TextStore
import yuku.alkitab.yes2.util.unsignedIntBinarySearch

fun main() {
    val store = TextStore(arrayOf("ignored", "visible"), 1)
    check(store.itemCount == 1)
    check(store.itemAt(0) == "visible")
    check(store.label(4) == "slot:4")
    check(add(2147483647, 1) == -2147483648)
    check(identity("imported") == "imported")
    check(unsignedIntBinarySearch(intArrayOf(0, 1, -1), -1) == 2)
    println("public multi-facade metadata consumer OK")
}
