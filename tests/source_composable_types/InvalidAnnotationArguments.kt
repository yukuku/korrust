package composetypeprobe
import androidx.compose.runtime.Composable
fun invalid(callback: @Composable(1) () -> Unit) {}
