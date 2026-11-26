package composeprobe
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
