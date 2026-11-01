package nestedcases
import library.Warehouse
import library.Probe
import library.Warehouse as Store

fun publicRead(): Int = Warehouse.Tools.total
fun aliasRead(): Int = Store.Tools.total
fun fullRead(): Int = library.Warehouse.Tools.Deep.count
fun wideRead(): Long = Warehouse.Tools.wide
fun labelRead(): String = Warehouse.Tools.label
fun invoke(): Int = Warehouse.Tools.plus(4)
fun fieldObject(): Probe = Warehouse.Shadow
fun fieldPriority(): Int = fieldObject().value
class LocalRoot(val Tools: LocalTools)
class LocalTools(val total: Int)
fun lexicalShadow(): Int {
    val Warehouse = LocalRoot(LocalTools(71))
    return Warehouse.Tools.total
}
fun classifierPriority(): Int = Warehouse.Shadow.value
