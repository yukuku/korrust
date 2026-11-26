package companionstateconsumer
import companionstate.Store
fun main() {
    val store = Store(2)
    check(store.read() == 6)
    Store.custom = 8
    check(Store.custom == 16)
    Store.raw = "consumer"
    check(Store.raw == "consumer")
    println("companion-consumer-ok")
}
