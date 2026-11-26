package markers.consumer
import markers.library.Key
import markers.library.Small
import markers.library.PairBox
import markers.library.NullableBox
import markers.library.reset
import markers.library.trace
import markers.library.textEvent
import markers.library.keyEvent
import markers.library.GenericBox
fun defaultBox(): PairBox = PairBox()
fun explicitBox(): PairBox = PairBox(Key(31L), Small(5), "given")
fun reordered(): PairBox = PairBox(text = "named", key = Key(41L))
fun nullableBox(): NullableBox = NullableBox()
fun genericBox(): GenericBox<String> = GenericBox(item = "generic")
fun qualified(): PairBox = markers.library.PairBox(text = "qualified")

fun numeric(): Long = defaultBox().key.raw + explicitBox().key.raw + reordered().key.raw
fun defaults(): String = defaultBox().text + ":" + reordered().text + ":" + genericBox().item + ":" + qualified().text
fun nullable(): Boolean = nullableBox().key == null
fun wide(): Long = markers.library.WideBox(p32 = 99).p0.raw + markers.library.WideBox().p33.raw
fun order(): String {
    reset()
    val value = markers.library.Ordered(text = textEvent("a"), key = keyEvent("b"))
    return trace() + ":" + value.text
}
fun evolved(): String {
    val value = markers.library.Evolved()
    return value.suffix + ":" + value.key.raw
}
