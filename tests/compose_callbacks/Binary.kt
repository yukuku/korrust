package composeprobe
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ComposableTarget
import androidx.compose.runtime.Stable

object BinaryEvents {
    var order: String = ""
    fun text(value: String): String { order += value; return value }
    fun number(value: Int): Int { order += "n"; return value }
}
@Stable fun stableAmount(value: Int = 9): Int = value
@Stable object StableReceiver
@Stable fun StableReceiver.stableAmount(value: Int = 9): Int = value
@Stable fun Registry.stableAmount(value: Int = 9): Int = value
@Composable
@ComposableTarget("fixture.Applier")
fun paint(amount: Int = 5, label: String? = "xy") {
    Events.hit(amount + (label?.length ?: 0))
}
@Composable fun many(
    p0: Int, p1: Int, p2: Int, p3: Int, p4: Int, p5: Int,
    p6: Int, p7: Int, p8: Int, p9: Int, p10: Int,
) { Events.hit(p0+p1+p2+p3+p4+p5+p6+p7+p8+p9+p10) }
@Composable fun <T> genericPaint(value: T) { Events.hit(1) }
@Composable fun resultPaint(): Int = 1
@Composable
@ComposableTarget("fixture.OtherApplier")
fun otherPaint() { Events.hit(100) }
object ObjectPaint {
    @JvmStatic @Composable fun show(value: Int) { Events.hit(value) }
}
class FixedRegistry {
    fun accept(block: @ComposableTarget("fixture.Applier") @Composable () -> Unit) {}
}

@Stable fun BinaryEvents.stableAmount(value: Int = 9): Int = value
class UnstablePayload(var value: Int)
@Stable fun unstablePayload(): UnstablePayload = UnstablePayload(9)
@Composable fun consumePayload(value: UnstablePayload) { Events.hit(value.value) }
@Composable fun pairedPayload(first: Int, value: UnstablePayload) { Events.hit(first + value.value) }
@Composable fun manyRuntime(
    p0: Int, p1: Int, p2: Int, p3: Int, p4: Int, p5: Int,
    p6: Int, p7: Int, p8: Int, p9: Int, p10: UnstablePayload,
) { Events.hit(p0+p1+p2+p3+p4+p5+p6+p7+p8+p9+p10.value) }
class GenericPayload<T>(val value: T)
fun genericPayload(): GenericPayload<String> = GenericPayload("x")
@Composable fun genericPayloadUse(value: GenericPayload<String>) { Events.hit(value.value.length) }
