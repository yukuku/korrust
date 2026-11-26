package composeprobe
import androidx.compose.runtime.SideEffect
import composeprobe.lookalike.accept as ordinaryAccept
fun stateless(registry: Registry) { registry.accept { Events.hit(1) } }
fun captured(registry: Registry, value: Int) { registry.accept { Events.hit(value) } }
fun throughInterface(listener: Listener, value: Int) { listener.supply { Events.hit(value) } }
fun throughTopLevel(registry: Registry, value: Int) { deliver(registry) { Events.hit(value) } }
fun mutable(registry: Registry) {
    var value = 3
    registry.accept { Events.hit(value); value += 1 }
}
fun wide(registry: Registry, text: String?, value: Long, fraction: Double) {
    registry.accept { Events.wide(text, value, fraction) }
}
fun localReturn(registry: Registry, value: Int) {
    registry.accept exit@ { Events.hit(value); return@exit }
}
fun exceptional(registry: Registry) { registry.accept { Events.fail() } }
fun lookalike() { ordinaryAccept { Events.hit(31) } }
fun binaryCaptured(registry: Registry, amount: Int) { registry.accept { paint(amount, "abc") } }
fun binaryStable(registry: Registry) { registry.accept { paint(stableAmount(), "abc") } }
fun binaryDefaults(registry: Registry) { registry.accept { paint() } }
fun binaryOrdered(registry: Registry) {
    registry.accept { paint(label = BinaryEvents.text("q"), amount = BinaryEvents.number(7)) }
}
fun binaryMany(registry: Registry) {
    registry.accept { many(0,1,2,3,4,5,6,7,8,9,10) }
}

class Controller : LexicalBase() {
    val initial: Int = activity
    fun constant(registry: Registry) { registry.accept { globalHit() } }
    fun ordinary(registry: Registry) {
        registry.apply { execute { Events.hit(activity) } }
    }
    fun install(registry: Registry) {
        registry.apply {
            accept {
                Events.hit(activity)
                if (this === registry) Events.hit(100)
            }
        }
    }
    fun shadow(shadow: ShadowRegistry) {
        shadow.apply { accept { Events.hit(activity) } }
    }
    fun nested(shadow: ShadowRegistry) {
        shadow.apply { registry.apply { accept { Events.hit(activity) } } }
    }
    fun labeled(shadow: ShadowRegistry) {
        shadow.apply { accept { Events.hit(this@Controller.activity) } }
    }
}

fun memoHost(registry: Registry, host: MemoHost?) {
    registry.accept {
        callbacks({ value -> host?.hit(value) }, { host?.hit(11) }, { host?.hit(13) })
    }
}
fun memoStable(registry: Registry, number: Int, text: String?) {
    registry.accept {
        callbacks({ value -> Events.hit(number + value); Events.hit(text?.length ?: 0) }, {}, ::globalHit)
    }
}
fun memoWide(registry: Registry, long: Long, double: Double, float: Float, flag: Boolean, char: Char, byte: Byte, short: Short) {
    registry.accept {
        callbacks({}, { MemoEvents.scalars(long, double, float, flag, char, byte, short) }, ::globalHit)
    }
}
fun memoEffect(registry: Registry) {
    registry.accept { SideEffect { Events.hit(53) } }
}
fun binaryStableReceiver(registry: Registry) { registry.accept { paint(StableReceiver.stableAmount(), "abc") } }
fun binaryDynamicReceiver(registry: Registry) { registry.accept { paint(registry.stableAmount(), "abc") } }
fun binaryUnstableReceiver(registry: Registry) { registry.accept { paint(BinaryEvents.stableAmount(), "abc") } }
fun binaryUnstableResult(registry: Registry) { registry.accept { consumePayload(unstablePayload()) } }
fun binaryRuntimeShift(registry: Registry) { registry.accept { pairedPayload(3, unstablePayload()) } }
fun binaryRuntimeMany(registry: Registry) { registry.accept { manyRuntime(0,1,2,3,4,5,6,7,8,9,unstablePayload()) } }

fun forwardTyped(registry: Registry, callback: @androidx.compose.runtime.Composable () -> Unit) { registry.accept(callback) }
