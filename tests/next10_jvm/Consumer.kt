package nextjvm.consumer

import nextjvm.api.answer
import nextjvm.api.Secret
import nextjvm.api.ObjectSecret
import nextjvm.cross.crossFile

fun main(args: Array<String>) {
    check(answer() == 42)
    check(crossFile() == 49)
    check(Secret.create(5).again(6).value == 6)
    val factory = Secret.create(0).factory()
    check(factory(7).value == 7)
    check(Secret.Nested().create(8).value == 8)
    check(ObjectSecret.Factory.create() != null)
    println("OK")
}
