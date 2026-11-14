package nextjvm.consumer
import nextjvm.api.ModuleOnly
fun main(args: Array<String>) {
    check(ModuleOnly(19).value == 19)
    println("FRIEND")
}
