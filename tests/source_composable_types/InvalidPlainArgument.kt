package composetypeprobe
import androidx.compose.runtime.Composable
fun invalid(registry: composeprobe.Registry, callback: () -> Unit) { registry.accept(callback) }
