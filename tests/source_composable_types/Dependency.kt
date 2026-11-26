package composeprobe
import androidx.compose.runtime.Composable
class Registry {
    var content: (@Composable () -> Unit)? = null
    fun accept(block: @Composable () -> Unit) { content = block }
}
