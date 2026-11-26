package composetypeprobe
import androidx.compose.runtime.Composable
fun invalid(callback: @Composable () -> Unit) { callback() }
