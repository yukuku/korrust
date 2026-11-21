package staticconsumer
import staticprobe.Host
import staticprobe.Global
fun main() {
    check(Host.first == "A")
    check(Host.Companion.first == "A")
    Host.mutable = null
    check(Host.mutable == null)
    Global.number = 12
    check(Global.number == 12)
    check(Host.Companion.deferred()() == "A")
    check(staticprobe.Cache.EMPTY.value == 4)
    staticprobe.Cache.boxed = 9
    check(staticprobe.Cache.boxed == 9)
    check(staticprobe.Cache.names == listOf("a", "b"))
    check(staticprobe.Cache.Companion.getEMPTY() == "method")
    println("OK")
}
