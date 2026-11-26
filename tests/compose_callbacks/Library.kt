package composeprobe
import androidx.compose.runtime.Composable

class Registry {
    var content: (@Composable () -> Unit)? = null
    fun accept(block: @Composable () -> Unit) { content = block }
}
interface Listener { fun supply(block: @Composable () -> Unit) }
fun deliver(registry: Registry, block: @Composable () -> Unit) { registry.accept(block) }
object Events {
    var value: Int = 0
    fun hit(value: Int) { this.value += value }
    fun wide(text: String?, value: Long, fraction: Double) {
        this.value += (text?.length ?: 0) + value.toInt() + fraction.toInt()
    }
    fun fail() { throw IllegalStateException("compose-body") }
}
class ArgumentSink { fun accept(block: @Composable (Int) -> Unit) {} }
class ResultSink { fun accept(block: @Composable () -> Int) {} }
class ReceiverSink { fun accept(block: @Composable String.() -> Unit) {} }
class NullableSink { fun accept(block: (@Composable () -> Unit)?) {} }
class NestedSink { fun accept(block: List<@Composable () -> Unit>) {} }
class InlineSink { inline fun accept(noinline block: @Composable () -> Unit) {} }
class GenericSink { fun <T> accept(value: T, block: @Composable () -> Unit) {} }
