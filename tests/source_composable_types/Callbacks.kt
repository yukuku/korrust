package composetypeprobe
import androidx.compose.runtime.Composable as Paint
import composeprobe.Registry
fun identity(callback: @Paint () -> Unit): @Paint () -> Unit = callback
fun qualified(callback: @androidx.compose.runtime.Composable () -> Unit): @androidx.compose.runtime.Composable () -> Unit = callback
fun parentheses(callback: @Paint() () -> Unit): @Paint () -> Unit = callback
fun wrapped(callback: @Paint () -> Unit): @Paint () -> Unit = { callback() }
fun explicitInvoke(callback: @Paint () -> Unit): @Paint () -> Unit = { callback.invoke() }
fun saved(callback: @Paint () -> Unit): @Paint () -> Unit {
    val local: @Paint () -> Unit = callback
    return { local() }
}
fun forward(registry: Registry, callback: @Paint () -> Unit) { registry.accept(callback) }
