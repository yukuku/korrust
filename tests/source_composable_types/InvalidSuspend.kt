package composetypeprobe
import androidx.compose.runtime.Composable
fun unsupported(callback: @Composable suspend () -> Unit) {}
