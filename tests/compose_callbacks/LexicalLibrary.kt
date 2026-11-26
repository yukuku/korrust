package composeprobe
import androidx.compose.runtime.Composable

fun execute(block: () -> Unit) { block() }
fun globalHit() { Events.hit(37) }
class ShadowRegistry(val registry: Registry) {
    val activity: Int = 41
    fun accept(block: @Composable () -> Unit) { registry.accept(block) }
}
