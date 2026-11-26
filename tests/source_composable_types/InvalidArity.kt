package composetypeprobe
import androidx.compose.runtime.Composable
fun invalid(callback: @Composable () -> Unit): @Composable () -> Unit = { callback(1) }
