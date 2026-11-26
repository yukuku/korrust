package whenjoinsconsumer
import whenjoins.nullable
import whenjoins.nullFirst
import whenjoins.boxed
import whenjoins.wide
fun main() {
    check(nullable(9) == null)
    check(nullable(0)!!.code == 10)
    check(nullFirst(0) == null)
    check(nullFirst(9)!!.code == 40)
    check(boxed(true) == 17 && boxed(false) == null)
    check(wide(true) == 4294967298L && wide(false) == null)
    println("when-consumer-ok")
}
