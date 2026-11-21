package consumer
import stable.dependency.Box
fun main() {
    check(stable.guarded(Box("A😀", null)) == 3)
    check(stable.chain(Box(null, Box("ok", null))) == 2)
    println("OK")
}
