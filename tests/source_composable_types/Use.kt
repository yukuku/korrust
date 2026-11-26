package composetypeconsumer
import androidx.compose.runtime.Composable
import composeprobe.Registry
import composetypeprobe.forward
import composetypeprobe.wrapped
fun fromConsumer(registry: Registry, callback: @Composable () -> Unit): @Composable () -> Unit {
    forward(registry, callback)
    return wrapped(callback)
}
